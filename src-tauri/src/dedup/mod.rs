use crate::db::videos::get_video_by_id;
use crate::models::{DuplicateGroup, DuplicateResolutionAction, ResolutionSummary, Video};
use rusqlite::Connection;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Normalizes Chinese full-width characters and punctuation to standard ASCII.
fn normalize_chinese_and_fullwidth(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '（' | '〔' => out.push('('),
            '）' | '〕' => out.push(')'),
            '【' => out.push('['),
            '】' => out.push(']'),
            '《' | '》' => out.push(' '),
            '\u{3000}' => out.push(' '), // Full-width space
            '－' => out.push('-'),
            '＿' => out.push('_'),
            '：' | '，' | '。' => out.push(' '),
            _ => out.push(c),
        }
    }
    out
}

/// Normalizes video file name for fuzzy similarity comparison.
/// Strips extension, lowercases, handles full-width punctuation, and removes common copy markers
/// (e.g. ` (1)`, ` - 副本`, `_copy`, `复件`). Also strips common quality/release tags (e.g. `1080p`).
pub fn normalize_file_name_for_fuzzy(name: &str) -> String {
    let stem = Path::new(name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(name);

    let stem_normalized = normalize_chinese_and_fullwidth(stem).to_lowercase();
    let mut cleaned = stem_normalized.trim().to_string();

    let explicit_prefixes = [
        "copy of ", "复件 ", "复件-", "复件_", "复件",
        "备份 ", "备份-", "备份_",
    ];

    let explicit_suffixes = [
        " - 副本", " -副本", "-副本", "_副本", " 副本", "副本",
        " - 备份", "-备份", "_备份", " 备份", "备份",
        " - copy", "-copy", "_copy", " copy", "copy",
    ];

    loop {
        let before = cleaned.clone();

        // 1. Strip explicit copy prefixes
        for pre in &explicit_prefixes {
            if cleaned.starts_with(pre) {
                let cand = cleaned[pre.len()..].trim().to_string();
                if cand.len() >= 2 {
                    cleaned = cand;
                }
            }
        }

        // 2. Strip explicit copy suffixes
        for suf in &explicit_suffixes {
            if cleaned.ends_with(suf) {
                let cand = cleaned[..cleaned.len() - suf.len()].trim().to_string();
                if cand.len() >= 2 {
                    cleaned = cand;
                }
            }
        }

        // 3. Strip bracket indices: (1), [1], (copy), (副本), (备份)
        for &(open, close) in &[('(', ')'), ('[', ']')] {
            if cleaned.ends_with(close) {
                if let Some(start) = cleaned.rfind(open) {
                    let inner = cleaned[start + 1..cleaned.len() - 1].trim();
                    let is_copy_index = (inner.len() <= 2 && inner.chars().all(|c| c.is_ascii_digit()))
                        || inner == "copy"
                        || inner == "副本"
                        || inner == "备份";
                    if is_copy_index {
                        let cand = cleaned[..start].trim().to_string();
                        if cand.len() >= 2 {
                            cleaned = cand;
                        }
                    }
                }
            }
        }

        if cleaned == before {
            break;
        }
    }

    // 4. Strip common quality/release tags & normalize separators
    let quality_tags = [
        "1080p", "720p", "4k", "2160p", "480p", "360p", "1080i", "uhd", "fhd", "hd",
        "bluray", "bdrip", "web-dl", "webrip", "hdtv", "dvdrip",
        "x264", "x265", "h264", "h265", "hevc",
    ];

    let words: Vec<&str> = cleaned
        .split(|c: char| c.is_whitespace() || c == '.' || c == '_' || c == '-' || c == '+')
        .filter(|w| !w.is_empty())
        .collect();

    let mut filtered = Vec::new();
    for w in words {
        let w_clean = w.trim_matches(|c: char| c == '(' || c == ')' || c == '[' || c == ']' || c == '{' || c == '}');
        if quality_tags.contains(&w_clean) {
            continue;
        }
        if !w_clean.is_empty() {
            filtered.push(w_clean);
        }
    }

    if filtered.is_empty() {
        cleaned.split_whitespace().collect::<Vec<_>>().join(" ")
    } else {
        filtered.join(" ")
    }
}

/// Checks whether a normalized filename is a common non-distinctive/generic name
/// (e.g. "video", "1", "clip", "未命名"). Generic names require stricter matching.
pub fn is_generic_name(name: &str) -> bool {
    let trimmed = name.trim();
    if trimmed.len() <= 2 {
        return true;
    }
    if trimmed.chars().all(|c| c.is_ascii_digit()) {
        return true;
    }

    let generic_words = [
        "video", "vid", "movie", "film", "clip", "test", "demo", "output", "temp",
        "record", "recording", "screen", "screencast", "cam", "camera", "untitled",
        "new video", "sample", "preview", "download",
        "新建视频", "未命名", "视频", "录像", "录屏", "无题", "未命名视频", "测试", "剪辑",
    ];

    for gw in &generic_words {
        if trimmed == *gw {
            return true;
        }
    }
    false
}

/// Evaluates whether two videos with the same normalized name are actually duplicate copies.
pub fn are_videos_similar(v1: &Video, v2: &Video, is_generic: bool) -> bool {
    // 1. If exact hash matches, they are definitively identical
    if !v1.file_hash.is_empty() && v1.file_hash == v2.file_hash {
        return true;
    }

    let min_size = v1.file_size.min(v2.file_size);
    let max_size = v1.file_size.max(v2.file_size);

    // 2. Generic names (e.g. "video", "1", "未命名"): require strict verification
    // to prevent unrelated clips from falsely colliding.
    if is_generic {
        if let (Some(dur1), Some(dur2)) = (v1.duration, v2.duration) {
            let dur_diff = (dur1 - dur2).abs();
            if dur_diff <= 0.5 {
                let size_diff_pct = (max_size - min_size) as f64 / (max_size.max(1) as f64);
                if size_diff_pct <= 0.05 {
                    if let ((Some(w1), Some(h1)), (Some(w2), Some(h2))) =
                        ((v1.width, v1.height), (v2.width, v2.height))
                    {
                        return w1 == w2 && h1 == h2;
                    }
                    return true;
                }
            }
        }
        return false;
    }

    // 3. Non-generic names:
    // Reject if sizes differ by > 20x (e.g. 10GB full movie vs 50MB thumbnail/sample)
    if min_size > 0 && max_size / min_size > 20 {
        return false;
    }

    // If both have durations, use adaptive tolerance:
    if let (Some(dur1), Some(dur2)) = (v1.duration, v2.duration) {
        if dur1 <= 0.0 || dur2 <= 0.0 {
            return min_size > 0 && (max_size - min_size) as f64 / max_size as f64 <= 0.15;
        }

        let dur_diff = (dur1 - dur2).abs();
        let max_dur = dur1.max(dur2);

        // Closeness criteria:
        // Short clips (<= 60s): difference within 2.0s or within 3%
        // Medium clips (<= 600s): difference within 3.5s or within 2%
        // Long movies (> 600s): difference within 6.0s or within 1%
        let dur_matches = if max_dur <= 60.0 {
            dur_diff <= 2.0 || (dur_diff / max_dur) <= 0.03
        } else if max_dur <= 600.0 {
            dur_diff <= 3.5 || (dur_diff / max_dur) <= 0.02
        } else {
            dur_diff <= 6.0 || (dur_diff / max_dur) <= 0.01
        };

        if !dur_matches {
            return false;
        }

        // Check aspect ratio orientation (landscape vs portrait)
        if let ((Some(w1), Some(h1)), (Some(w2), Some(h2))) =
            ((v1.width, v1.height), (v2.width, v2.height))
        {
            if w1 > 0 && h1 > 0 && w2 > 0 && h2 > 0 {
                let v1_portrait = h1 > w1 * 12 / 10;
                let v2_portrait = h2 > w2 * 12 / 10;
                // If one is portrait and other is landscape, only match if duration is virtually identical (<= 0.5s)
                if v1_portrait != v2_portrait && dur_diff > 0.5 {
                    return false;
                }
            }
        }

        return true;
    }

    // If one or both lack duration (e.g. ffprobe pending or failed):
    // Fall back to size closeness or exact size
    if v1.file_size == v2.file_size && v1.file_size > 0 {
        return true;
    }

    if min_size > 0 && (max_size - min_size) as f64 / max_size as f64 <= 0.10 {
        return true;
    }

    false
}

/// Discovers duplicate video groups from database according to the given strategy.
/// Supported strategies:
/// - "exact_hash": Byte-for-byte exact duplicate (Identical file_hash & file_size)
/// - "metadata": Stream specs match (Identical duration rounded to 0.5s, width, height, codec)
/// - "fuzzy_name": Adaptive normalized name similarity + intelligent duration/size clustering
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
                is_starred: false,
                watch_status: "unwatched".to_string(),
                playback_progress: 0.0,
                color_tag: None,
                last_played_at: None,
                tags: Vec::new(),
            })
        })
        .map_err(|e| format!("Failed querying videos: {}", e))?;

    let mut all_videos = Vec::new();
    for v in video_rows {
        all_videos.push(v.map_err(|e| e.to_string())?);
    }

    let mut result_groups = Vec::new();
    let mut group_counter = 1;

    match strategy {
        "fuzzy_name" => {
            // Group videos by normalized filename
            let mut name_map: HashMap<String, Vec<Video>> = HashMap::new();
            for v in all_videos {
                let norm_name = normalize_file_name_for_fuzzy(&v.file_name);
                if norm_name.len() < 2 {
                    continue;
                }
                name_map.entry(norm_name).or_default().push(v);
            }

            for (norm_name, group_videos) in name_map {
                let n = group_videos.len();
                if n < 2 {
                    continue;
                }

                let is_generic = is_generic_name(&norm_name);

                // Build adjacency list for connected-component clustering
                let mut adj = vec![vec![]; n];
                for i in 0..n {
                    for j in (i + 1)..n {
                        if are_videos_similar(&group_videos[i], &group_videos[j], is_generic) {
                            adj[i].push(j);
                            adj[j].push(i);
                        }
                    }
                }

                let mut visited = vec![false; n];
                for i in 0..n {
                    if visited[i] {
                        continue;
                    }

                    let mut component = Vec::new();
                    let mut queue = std::collections::VecDeque::new();
                    visited[i] = true;
                    queue.push_back(i);

                    while let Some(curr) = queue.pop_front() {
                        component.push(group_videos[curr].clone());
                        for &nbr in &adj[curr] {
                            if !visited[nbr] {
                                visited[nbr] = true;
                                queue.push_back(nbr);
                            }
                        }
                    }

                    if component.len() > 1 {
                        // Sort videos within the group:
                        // 1. Higher resolution first
                        // 2. Larger size first
                        // 3. Newest updated_at first
                        component.sort_by(|a, b| {
                            let res_a = a.width.unwrap_or(0) * a.height.unwrap_or(0);
                            let res_b = b.width.unwrap_or(0) * b.height.unwrap_or(0);
                            res_b
                                .cmp(&res_a)
                                .then_with(|| b.file_size.cmp(&a.file_size))
                                .then_with(|| b.updated_at.cmp(&a.updated_at))
                        });

                        let total_size: u64 = component.iter().map(|v| v.file_size).sum();
                        let keep_size = component[0].file_size;
                        let reclaimable_size = total_size.saturating_sub(keep_size);

                        result_groups.push(DuplicateGroup {
                            group_id: format!("grp_{}_fuzzy_{}", group_counter, norm_name),
                            match_reason: "fuzzy_name".to_string(),
                            match_label: format!("同名副本 (主体: {})", norm_name),
                            total_size,
                            reclaimable_size,
                            videos: component,
                        });
                        group_counter += 1;
                    }
                }
            }
        }

        "metadata" => {
            let mut group_map: HashMap<String, Vec<Video>> = HashMap::new();
            for v in all_videos {
                if let (Some(dur), Some(w), Some(h)) = (v.duration, v.width, v.height) {
                    if dur > 1.0 && w > 0 && h > 0 {
                        let dur_bucket = (dur * 2.0).round() as i64; // 0.5s precision
                        let codec_str = v.codec.as_deref().unwrap_or("unknown").to_lowercase();
                        let key = format!("meta_{}x{}_{}_{}", w, h, dur_bucket, codec_str);
                        group_map.entry(key).or_default().push(v);
                    }
                }
            }

            for (key, mut group_videos) in group_map {
                if group_videos.len() > 1 {
                    group_videos.sort_by(|a, b| {
                        let res_a = a.width.unwrap_or(0) * a.height.unwrap_or(0);
                        let res_b = b.width.unwrap_or(0) * b.height.unwrap_or(0);
                        res_b
                            .cmp(&res_a)
                            .then_with(|| b.file_size.cmp(&a.file_size))
                            .then_with(|| b.updated_at.cmp(&a.updated_at))
                    });

                    let total_size: u64 = group_videos.iter().map(|v| v.file_size).sum();
                    let keep_size = group_videos[0].file_size;
                    let reclaimable_size = total_size.saturating_sub(keep_size);

                    result_groups.push(DuplicateGroup {
                        group_id: format!("grp_{}_{}", group_counter, key),
                        match_reason: "metadata".to_string(),
                        match_label: format!(
                            "规格一致 ({}x{}, {})",
                            group_videos[0].width.unwrap_or(0),
                            group_videos[0].height.unwrap_or(0),
                            group_videos[0].codec.as_deref().unwrap_or("未知")
                        ),
                        total_size,
                        reclaimable_size,
                        videos: group_videos,
                    });
                    group_counter += 1;
                }
            }
        }

        _ => {
            // Default: "exact_hash"
            let mut group_map: HashMap<String, Vec<Video>> = HashMap::new();
            for v in all_videos {
                if !v.file_hash.is_empty() && v.file_size > 0 {
                    let key = format!("hash_{}_{}", v.file_size, v.file_hash);
                    group_map.entry(key).or_default().push(v);
                }
            }

            for (key, mut group_videos) in group_map {
                if group_videos.len() > 1 {
                    group_videos.sort_by(|a, b| {
                        let res_a = a.width.unwrap_or(0) * a.height.unwrap_or(0);
                        let res_b = b.width.unwrap_or(0) * b.height.unwrap_or(0);
                        res_b
                            .cmp(&res_a)
                            .then_with(|| b.file_size.cmp(&a.file_size))
                            .then_with(|| b.updated_at.cmp(&a.updated_at))
                    });

                    let total_size: u64 = group_videos.iter().map(|v| v.file_size).sum();
                    let keep_size = group_videos[0].file_size;
                    let reclaimable_size = total_size.saturating_sub(keep_size);

                    result_groups.push(DuplicateGroup {
                        group_id: format!("grp_{}_{}", group_counter, key),
                        match_reason: "exact_hash".to_string(),
                        match_label: format!("完全一致 (哈希采样吻合, {} 份)", group_videos.len()),
                        total_size,
                        reclaimable_size,
                        videos: group_videos,
                    });
                    group_counter += 1;
                }
            }
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
        // Basic: strip extension and lowercase
        assert_eq!(normalize_file_name_for_fuzzy("MyVideo.mp4"), "myvideo");

        // Copy marker suffixes — should be stripped
        assert_eq!(normalize_file_name_for_fuzzy("MyVideo - 副本.mp4"), "myvideo");
        assert_eq!(normalize_file_name_for_fuzzy("MyVideo 副本.mp4"), "myvideo");
        assert_eq!(normalize_file_name_for_fuzzy("MyVideo_副本.mp4"), "myvideo");
        assert_eq!(normalize_file_name_for_fuzzy("MyVideo_copy.mp4"), "myvideo");
        assert_eq!(normalize_file_name_for_fuzzy("MyVideo-copy.mp4"), "myvideo");
        assert_eq!(normalize_file_name_for_fuzzy("MyVideo copy.mp4"), "myvideo");
        assert_eq!(normalize_file_name_for_fuzzy("copy of myvideo.mp4"), "myvideo");
        assert_eq!(normalize_file_name_for_fuzzy("复件 MyVideo.mp4"), "myvideo");

        // Bracket index — 1-2 digits should be stripped
        assert_eq!(normalize_file_name_for_fuzzy("MyVideo (1).mp4"), "myvideo");
        assert_eq!(normalize_file_name_for_fuzzy("MyVideo (2).mp4"), "myvideo");
        assert_eq!(normalize_file_name_for_fuzzy("MyVideo[1].mp4"), "myvideo");
        assert_eq!(normalize_file_name_for_fuzzy("MyVideo（1）.mp4"), "myvideo");

        // Multi-layer nested copy markers — should all be stripped
        assert_eq!(normalize_file_name_for_fuzzy("MyVideo - 副本 (1).mp4"), "myvideo");

        // Quality/resolution tag stripping
        assert_eq!(normalize_file_name_for_fuzzy("MyVideo.1080p.mp4"), "myvideo");
        assert_eq!(normalize_file_name_for_fuzzy("MyVideo.720p.mkv"), "myvideo");

        // Title with year and tags
        assert_eq!(normalize_file_name_for_fuzzy("大鱼海棠_2016.mp4"), "大鱼海棠 2016");
        assert_eq!(normalize_file_name_for_fuzzy("大鱼海棠 (2016) [1080p].mp4"), "大鱼海棠 2016");

        // Separator normalization
        assert_eq!(normalize_file_name_for_fuzzy("Episode_01.mkv"), "episode 01");
        assert_eq!(normalize_file_name_for_fuzzy("Avengers_4.mp4"), "avengers 4");
        assert_eq!(normalize_file_name_for_fuzzy("ab_1.mp4"), "ab 1");
    }

    #[test]
    fn test_find_fuzzy_name_duplicates() {
        let conn = setup_test_db();
        conn.execute(
            "INSERT INTO videos (file_path, file_name, file_hash, file_size, duration, width, height, codec, created_at, updated_at, status)
             VALUES ('/path/vlog.mp4', 'vlog.mp4', 'hash_v1', 50000000, 60.2, 1920, 1080, 'h264', 100, 100, 'ready'),
                    ('/path/vlog_copy.mp4', 'vlog - 副本.mp4', 'hash_v2', 49900000, 60.4, 1920, 1080, 'h264', 200, 200, 'ready'),
                    ('/path/other.mp4', 'other.mp4', 'hash_v3', 50000000, 60.3, 1920, 1080, 'h264', 300, 300, 'ready')",
            [],
        ).unwrap();

        let duplicates = find_duplicates(&conn, "fuzzy_name").unwrap();
        assert_eq!(duplicates.len(), 1);
        assert_eq!(duplicates[0].videos.len(), 2);
        assert_eq!(duplicates[0].match_reason, "fuzzy_name");
        assert!(duplicates[0].match_label.contains("vlog"));
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
