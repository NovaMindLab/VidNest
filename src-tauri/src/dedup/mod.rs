use crate::db::videos::get_video_by_id;
use crate::models::{DuplicateGroup, DuplicateResolutionAction, ResolutionSummary, Video};
use rusqlite::Connection;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Normalizes video file name for fuzzy similarity comparison.
/// Strips extension, lowercases, and strips common duplicate suffixes like ` (1)`, ` - 副本`, `_copy`.
pub fn normalize_file_name_for_fuzzy(name: &str) -> String {
    let stem = Path::new(name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(name)
        .to_lowercase();

    let mut cleaned = stem.trim().to_string();

    // Strip common duplicate suffixes iteratively
    let patterns = [
        " - 副本",
        " 副本",
        "-副本",
        "_副本",
        "_copy",
        "-copy",
        " copy",
    ];

    for pat in patterns {
        if let Some(pos) = cleaned.rfind(pat) {
            cleaned.truncate(pos);
            cleaned = cleaned.trim().to_string();
        }
    }

    // Strip trailing regex-like patterns e.g. " (1)", " (2)", "_1", "_2", " (copy)"
    if let Some(open_paren) = cleaned.rfind('(') {
        if cleaned.ends_with(')') {
            let inner = &cleaned[open_paren + 1..cleaned.len() - 1];
            if inner.chars().all(|c| c.is_ascii_digit()) || inner == "copy" || inner == "副本" {
                cleaned.truncate(open_paren);
                cleaned = cleaned.trim().to_string();
            }
        }
    }

    // Strip trailing "_1", "_2"
    if let Some(idx) = cleaned.rfind('_') {
        let suffix = &cleaned[idx + 1..];
        if !suffix.is_empty() && suffix.chars().all(|c| c.is_ascii_digit()) {
            cleaned.truncate(idx);
            cleaned = cleaned.trim().to_string();
        }
    }

    cleaned
}

/// Discovers duplicate video groups from database according to the given strategy.
/// Supported strategies:
/// - "exact_hash": Byte-for-byte exact duplicate (Identical file_hash & file_size)
/// - "metadata": Stream specs match (Identical duration rounded to 0.5s, width, height, codec)
/// - "fuzzy_name": Normalized file stem match + identical duration
pub fn find_duplicates(conn: &Connection, strategy: &str) -> Result<Vec<DuplicateGroup>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, file_path, file_name, file_hash, file_size, duration, width, height, codec, created_at, updated_at, status, thumbnail_path
             FROM videos
             WHERE status = 'ready' OR status = 'indexed'",
        )
        .map_err(|e| format!("Failed to prepare query: {}", e))?;

    let video_rows = stmt
        .query_map([], |row| {
            let file_size: i64 = row.get(4)?;
            Ok(Video {
                id: row.get(0)?,
                file_path: row.get(1)?,
                file_name: row.get(2)?,
                file_hash: row.get(3)?,
                file_size: file_size as u64,
                duration: row.get(5)?,
                width: row.get(6)?,
                height: row.get(7)?,
                codec: row.get(8)?,
                created_at: row.get(9)?,
                updated_at: row.get(10)?,
                status: row.get(11)?,
                thumbnail_path: row.get(12)?,
                thumbnail_data: None,
            })
        })
        .map_err(|e| format!("Failed querying videos: {}", e))?;

    let mut all_videos = Vec::new();
    for v in video_rows {
        all_videos.push(v.map_err(|e| e.to_string())?);
    }

    let mut group_map: HashMap<String, Vec<Video>> = HashMap::new();

    match strategy {
        "metadata" => {
            for v in all_videos {
                // Must have duration and resolution to be safely matched by specs
                if let (Some(dur), Some(w), Some(h)) = (v.duration, v.width, v.height) {
                    if dur > 1.0 && w > 0 && h > 0 {
                        let dur_bucket = (dur * 2.0).round() as i64; // 0.5s precision
                        let codec_str = v.codec.as_deref().unwrap_or("unknown").to_lowercase();
                        let key = format!("meta_{}x{}_{}_{}", w, h, dur_bucket, codec_str);
                        group_map.entry(key).or_default().push(v);
                    }
                }
            }
        }
        "fuzzy_name" => {
            for v in all_videos {
                let norm_name = normalize_file_name_for_fuzzy(&v.file_name);
                if !norm_name.is_empty() {
                    let dur_bucket = v.duration.map(|d| (d * 2.0).round() as i64).unwrap_or(0);
                    let key = format!("fuzzy_{}_{}", norm_name, dur_bucket);
                    group_map.entry(key).or_default().push(v);
                }
            }
        }
        _ => {
            // Default: "exact_hash"
            for v in all_videos {
                if !v.file_hash.is_empty() && v.file_size > 0 {
                    let key = format!("hash_{}_{}", v.file_size, v.file_hash);
                    group_map.entry(key).or_default().push(v);
                }
            }
        }
    }

    let mut result_groups = Vec::new();
    let mut group_counter = 1;

    for (key, mut group_videos) in group_map {
        if group_videos.len() > 1 {
            // Sort videos within the group:
            // 1. Higher resolution first
            // 2. Larger size first
            // 3. Newest updated_at first
            group_videos.sort_by(|a, b| {
                let res_a = a.width.unwrap_or(0) * a.height.unwrap_or(0);
                let res_b = b.width.unwrap_or(0) * b.height.unwrap_or(0);
                res_b
                    .cmp(&res_a)
                    .then_with(|| b.file_size.cmp(&a.file_size))
                    .then_with(|| b.updated_at.cmp(&a.updated_at))
            });

            let total_size: u64 = group_videos.iter().map(|v| v.file_size).sum();
            // Primary recommendation is the first item; reclaimable space is all other items
            let keep_size = group_videos[0].file_size;
            let reclaimable_size = total_size.saturating_sub(keep_size);

            let (reason, label) = match strategy {
                "metadata" => (
                    "metadata".to_string(),
                    format!(
                        "规格一致 ({}x{}, {})",
                        group_videos[0].width.unwrap_or(0),
                        group_videos[0].height.unwrap_or(0),
                        group_videos[0].codec.as_deref().unwrap_or("未知")
                    ),
                ),
                "fuzzy_name" => (
                    "fuzzy_name".to_string(),
                    format!("同名副本 (主体: {})", normalize_file_name_for_fuzzy(&group_videos[0].file_name)),
                ),
                _ => (
                    "exact_hash".to_string(),
                    format!("完全一致 (哈希采样吻合, {} 份)", group_videos.len()),
                ),
            };

            result_groups.push(DuplicateGroup {
                group_id: format!("grp_{}_{}", group_counter, key),
                match_reason: reason,
                match_label: label,
                total_size,
                reclaimable_size,
                videos: group_videos,
            });
            group_counter += 1;
        }
    }

    // Sort groups descending by reclaimable_size so largest disk savings appear first
    result_groups.sort_by(|a, b| b.reclaimable_size.cmp(&a.reclaimable_size));

    Ok(result_groups)
}

/// Executes duplicate resolution actions in a batch transaction:
/// - Keeps the designated `keep_video_id`
/// - Merges metadata (sources, tags, analysis) from deleted videos to `keep_video_id`
/// - Deletes removed video records from database and FTS5
/// - If `delete_disk_file` is true, deletes physical video file from filesystem
pub fn resolve_duplicates(
    conn: &mut Connection,
    actions: Vec<DuplicateResolutionAction>,
) -> Result<ResolutionSummary, String> {
    let tx = conn
        .transaction()
        .map_err(|e| format!("Failed to begin transaction: {}", e))?;

    let mut groups_processed = 0;
    let mut videos_removed = 0;
    let mut bytes_reclaimed = 0u64;

    for action in actions {
        if action.delete_video_ids.is_empty() {
            continue;
        }

        let keep_id = action.keep_video_id;

        // Verify keep video exists
        let keep_video = match get_video_by_id(&tx, keep_id)? {
            Some(v) => v,
            None => return Err(format!("保留的目标视频 ID={} 不存在", keep_id)),
        };

        for del_id in action.delete_video_ids {
            if del_id == keep_id {
                continue; // Guard against accidental self-deletion
            }

            let del_video = match get_video_by_id(&tx, del_id)? {
                Some(v) => v,
                None => continue,
            };

            // 1. Merge video_sources
            let _ = tx.execute(
                "UPDATE video_sources SET video_id = ?1 WHERE video_id = ?2",
                rusqlite::params![keep_id, del_id],
            );

            // 2. Merge ai_analyses (tags and summary)
            let keep_has_ai: bool = tx
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM ai_analyses WHERE video_id = ?1)",
                    [keep_id],
                    |row| row.get(0),
                )
                .unwrap_or(false);

            if !keep_has_ai {
                // If keep video has no AI analysis, transfer del_video's record
                let _ = tx.execute(
                    "UPDATE ai_analyses SET video_id = ?1 WHERE video_id = ?2",
                    rusqlite::params![keep_id, del_id],
                );
            } else {
                // If both have AI analyses, merge tags
                let del_tags: Option<String> = tx
                    .query_row(
                        "SELECT tags FROM ai_analyses WHERE video_id = ?1",
                        [del_id],
                        |row| row.get(0),
                    )
                    .ok();

                if let Some(dt) = del_tags {
                    if !dt.is_empty() {
                        let keep_tags: String = tx
                            .query_row(
                                "SELECT tags FROM ai_analyses WHERE video_id = ?1",
                                [keep_id],
                                |row| row.get(0),
                            )
                            .unwrap_or_default();

                        let mut combined_tags: Vec<String> = keep_tags
                            .split(',')
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect();

                        for tag in dt.split(',') {
                            let clean = tag.trim().to_string();
                            if !clean.is_empty() && !combined_tags.contains(&clean) {
                                combined_tags.push(clean);
                            }
                        }

                        let merged_str = combined_tags.join(",");
                        let _ = tx.execute(
                            "UPDATE ai_analyses SET tags = ?1 WHERE video_id = ?2",
                            rusqlite::params![merged_str, keep_id],
                        );
                    }
                }
                let _ = tx.execute("DELETE FROM ai_analyses WHERE video_id = ?1", [del_id]);
            }

            // 3. Delete from database & FTS index
            tx.execute("DELETE FROM videos WHERE id = ?1", [del_id])
                .map_err(|e| format!("从数据库删除视频失败 ID={}: {}", del_id, e))?;

            let _ = tx.execute("DELETE FROM fts_index WHERE video_id = ?1", [del_id.to_string()]);

            // 4. Remove thumbnail if exists
            if let Some(thumb) = &del_video.thumbnail_path {
                let thumb_path = Path::new(thumb);
                if thumb_path.exists() {
                    let _ = fs::remove_file(thumb_path);
                }
            }

            // 5. Remove physical file from disk if requested
            if action.delete_disk_file {
                let file_p = Path::new(&del_video.file_path);
                if file_p.exists() {
                    let _ = fs::remove_file(file_p);
                }
            }

            bytes_reclaimed += del_video.file_size;
            videos_removed += 1;
        }

        // Re-index keep video in FTS5
        let _ = tx.execute(
            "DELETE FROM fts_index WHERE video_id = ?1",
            [keep_id.to_string()],
        );
        let _ = tx.execute(
            "INSERT INTO fts_index (video_id, title, subtitles, summary, tags) VALUES (?1, ?2, '', '', '')",
            rusqlite::params![keep_id.to_string(), keep_video.file_name],
        );

        groups_processed += 1;
    }

    tx.commit()
        .map_err(|e| format!("Failed to commit deduplication transaction: {}", e))?;

    Ok(ResolutionSummary {
        groups_processed,
        videos_removed,
        bytes_reclaimed,
        success: true,
        message: format!(
            "成功处理 {} 组重复视频，清理 {} 个冗余副本，释放约 {:.2} MB 空间",
            groups_processed,
            videos_removed,
            bytes_reclaimed as f64 / (1024.0 * 1024.0)
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migrations::apply_all;

    fn setup_test_db() -> Connection {
        let conn = Connection::open_in_memory().expect("open memory db");
        apply_all(&conn).expect("apply migrations");
        conn
    }

    #[test]
    fn test_normalize_file_name_for_fuzzy() {
        assert_eq!(normalize_file_name_for_fuzzy("MyVideo.mp4"), "myvideo");
        assert_eq!(normalize_file_name_for_fuzzy("MyVideo - 副本.mp4"), "myvideo");
        assert_eq!(normalize_file_name_for_fuzzy("MyVideo (1).mp4"), "myvideo");
        assert_eq!(normalize_file_name_for_fuzzy("MyVideo (2).mp4"), "myvideo");
        assert_eq!(normalize_file_name_for_fuzzy("MyVideo_copy.mp4"), "myvideo");
        assert_eq!(normalize_file_name_for_fuzzy("MyVideo_1.mp4"), "myvideo");
    }

    #[test]
    fn test_find_exact_hash_duplicates() {
        let conn = setup_test_db();
        // Insert 2 identical videos (same hash, same size)
        conn.execute(
            "INSERT INTO videos (file_path, file_name, file_hash, file_size, duration, width, height, codec, created_at, updated_at, status)
             VALUES ('/path/a.mp4', 'a.mp4', 'hash123', 1000000, 10.5, 1920, 1080, 'h264', 100, 100, 'ready'),
                    ('/path/b.mp4', 'b.mp4', 'hash123', 1000000, 10.5, 1920, 1080, 'h264', 200, 200, 'ready'),
                    ('/path/c.mp4', 'c.mp4', 'hash456', 2000000, 20.0, 1280, 720, 'h264', 300, 300, 'ready')",
            [],
        ).unwrap();

        let duplicates = find_duplicates(&conn, "exact_hash").unwrap();
        assert_eq!(duplicates.len(), 1);
        assert_eq!(duplicates[0].videos.len(), 2);
        assert_eq!(duplicates[0].match_reason, "exact_hash");
        assert_eq!(duplicates[0].reclaimable_size, 1000000);
    }

    #[test]
    fn test_find_metadata_duplicates() {
        let conn = setup_test_db();
        conn.execute(
            "INSERT INTO videos (file_path, file_name, file_hash, file_size, duration, width, height, codec, created_at, updated_at, status)
             VALUES ('/path/v1.mp4', 'v1.mp4', 'hash_different_1', 1200000, 60.1, 1920, 1080, 'h264', 100, 100, 'ready'),
                    ('/path/v2.mkv', 'v2.mkv', 'hash_different_2', 1100000, 60.2, 1920, 1080, 'h264', 200, 200, 'ready')",
            [],
        ).unwrap();

        let duplicates = find_duplicates(&conn, "metadata").unwrap();
        assert_eq!(duplicates.len(), 1);
        assert_eq!(duplicates[0].videos.len(), 2);
        assert_eq!(duplicates[0].match_reason, "metadata");
    }

    #[test]
    fn test_resolve_duplicates_and_merge_metadata() {
        let mut conn = setup_test_db();
        conn.execute(
            "INSERT INTO videos (id, file_path, file_name, file_hash, file_size, duration, width, height, codec, created_at, updated_at, status)
             VALUES (1, '/path/orig.mp4', 'orig.mp4', 'hash1', 5000, 10.0, 1920, 1080, 'h264', 100, 100, 'ready'),
                    (2, '/path/copy.mp4', 'copy.mp4', 'hash1', 5000, 10.0, 1920, 1080, 'h264', 200, 200, 'ready')",
            [],
        ).unwrap();

        // Add video_source and ai_analyses to copy video
        conn.execute(
            "INSERT INTO video_sources (video_id, original_url, platform) VALUES (2, 'https://youtube.com/watch?v=test', 'youtube')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO ai_analyses (video_id, summary, keywords, tags, model_name, created_at)
             VALUES (2, 'Summary of test', 'test', 'vlog,travel', 'local-ai', 200)",
            [],
        ).unwrap();

        let action = DuplicateResolutionAction {
            group_id: "grp_1".to_string(),
            keep_video_id: 1,
            delete_video_ids: vec![2],
            delete_disk_file: false,
        };

        let summary = resolve_duplicates(&mut conn, vec![action]).unwrap();
        assert_eq!(summary.groups_processed, 1);
        assert_eq!(summary.videos_removed, 1);
        assert_eq!(summary.bytes_reclaimed, 5000);

        // Verify video 2 is removed, video 1 still exists
        let v1 = get_video_by_id(&conn, 1).unwrap();
        assert!(v1.is_some());
        let v2 = get_video_by_id(&conn, 2).unwrap();
        assert!(v2.is_none());

        // Verify video_sources transferred to video 1
        let source_count: i64 = conn.query_row(
            "SELECT count(*) FROM video_sources WHERE video_id = 1",
            [],
            |r| r.get(0),
        ).unwrap();
        assert_eq!(source_count, 1);

        // Verify ai_analyses transferred to video 1
        let ai_count: i64 = conn.query_row(
            "SELECT count(*) FROM ai_analyses WHERE video_id = 1",
            [],
            |r| r.get(0),
        ).unwrap();
        assert_eq!(ai_count, 1);
    }
}
