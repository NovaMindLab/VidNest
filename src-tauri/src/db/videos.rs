use crate::models::{Video, VideoFilter, VideoListResponse};
use rusqlite::{params, Connection, Result};
use std::fs;
use std::path::Path;

pub fn insert_or_update_video(
    conn: &Connection,
    file_path: &str,
    file_name: &str,
    file_hash: &str,
    file_size: u64,
    duration: Option<f64>,
    width: Option<u32>,
    height: Option<u32>,
    codec: Option<&str>,
    thumbnail_path: Option<&str>,
    now_ms: i64,
) -> Result<i64, String> {
    // Check if video with file_path already exists
    let existing_id: Option<i64> = conn
        .query_row(
            "SELECT id FROM videos WHERE file_path = ?1",
            [file_path],
            |row| row.get(0),
        )
        .ok();

    if let Some(id) = existing_id {
        conn.execute(
            "UPDATE videos SET
                file_name = ?1,
                file_hash = ?2,
                file_size = ?3,
                duration = ?4,
                width = ?5,
                height = ?6,
                codec = ?7,
                thumbnail_path = ?8,
                updated_at = ?9
             WHERE id = ?10",
            params![
                file_name,
                file_hash,
                file_size as i64,
                duration,
                width,
                height,
                codec,
                thumbnail_path,
                now_ms,
                id
            ],
        )
        .map_err(|e| format!("Failed to update video record: {}", e))?;

        // Update FTS5 virtual table
        let _ = conn.execute(
            "DELETE FROM fts_index WHERE video_id = ?1",
            [id.to_string()],
        );
        let _ = conn.execute(
            "INSERT INTO fts_index (video_id, title, subtitles, summary, tags) VALUES (?1, ?2, '', '', '')",
            params![id.to_string(), file_name],
        );

        Ok(id)
    } else {
        conn.execute(
            "INSERT INTO videos (
                file_path, file_name, file_hash, file_size,
                duration, width, height, codec, thumbnail_path,
                created_at, updated_at, status
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 'ready')",
            params![
                file_path,
                file_name,
                file_hash,
                file_size as i64,
                duration,
                width,
                height,
                codec,
                thumbnail_path,
                now_ms,
                now_ms
            ],
        )
        .map_err(|e| format!("Failed to insert video record: {}", e))?;

        let id = conn.last_insert_rowid();

        // Index in FTS5
        let _ = conn.execute(
            "INSERT INTO fts_index (video_id, title, subtitles, summary, tags) VALUES (?1, ?2, '', '', '')",
            params![id.to_string(), file_name],
        );

        Ok(id)
    }
}

pub fn update_video_metadata(
    conn: &Connection,
    id: i64,
    duration: Option<f64>,
    width: Option<u32>,
    height: Option<u32>,
    codec: Option<&str>,
    thumbnail_path: Option<&str>,
) -> Result<(), String> {
    conn.execute(
        "UPDATE videos SET
            duration = COALESCE(?1, duration),
            width = COALESCE(?2, width),
            height = COALESCE(?3, height),
            codec = COALESCE(?4, codec),
            thumbnail_path = COALESCE(?5, thumbnail_path)
         WHERE id = ?6",
        params![duration, width, height, codec, thumbnail_path, id],
    )
    .map_err(|e| format!("Failed to update video metadata: {}", e))?;
    Ok(())
}

pub fn list_videos(conn: &Connection, filter: VideoFilter) -> Result<VideoListResponse, String> {
    let page = filter.page.unwrap_or(1).max(1);
    let page_size = filter.page_size.unwrap_or(100).clamp(1, 5000);
    let offset = (page - 1) * page_size;

    let mut conditions = Vec::new();
    let mut param_values = Vec::new();

    if let Some(search_text) = filter.search.as_ref().map(|s| s.trim()).filter(|s| !s.is_empty()) {
        // Query FTS5 matching IDs or fallback to filename like
        let search_pattern = format!("%{}%", search_text);
        conditions.push(
            "(id IN (SELECT CAST(video_id AS INTEGER) FROM fts_index WHERE fts_index MATCH ?1) OR file_name LIKE ?2)"
                .to_string(),
        );
        // Clean FTS term
        let fts_query = format!("\"{}\"*", search_text.replace('"', ""));
        param_values.push(fts_query);
        param_values.push(search_pattern);
    }

    if let Some(codec_text) = filter.codec.as_ref().map(|s| s.trim()).filter(|s| !s.is_empty()) {
        let idx = param_values.len() + 1;
        conditions.push(format!("codec = ?{}", idx));
        param_values.push(codec_text.to_uppercase());
    }

    // format_category
    if let Some(cat) = filter.format_category.as_deref().filter(|s| !s.is_empty() && *s != "all") {
        match cat {
            "shorts" => {
                conditions.push("((duration IS NOT NULL AND duration <= 90) OR (width IS NOT NULL AND height IS NOT NULL AND width > 0 AND height > 0 AND width < height))".to_string());
            }
            "movies" => {
                conditions.push("(duration >= 2700 AND (width IS NULL OR height IS NULL OR width >= height))".to_string());
            }
            "series" => {
                conditions.push("(file_name LIKE '%E0%' OR file_name LIKE '%E1%' OR file_name LIKE '%EP%' OR file_name LIKE '%第%集%' OR file_name LIKE '%第%话%' OR file_name LIKE '%S0%')".to_string());
            }
            "screen" => {
                conditions.push("(file_name LIKE '%Screen%' OR file_name LIKE '%录屏%' OR file_name LIKE '%截屏%' OR file_name LIKE '%OBS%' OR file_name LIKE '%Record%')".to_string());
            }
            _ => {}
        }
    }

    // quality
    if let Some(q) = filter.quality.as_deref().filter(|s| !s.is_empty() && *s != "all") {
        match q {
            "4k" => {
                conditions.push("(width >= 3840 OR height >= 2160)".to_string());
            }
            "1080p" => {
                conditions.push("((width >= 1920 AND width < 3840) OR (height >= 1080 AND height < 2160))".to_string());
            }
            "720p" => {
                conditions.push("((width >= 1280 AND width < 1920) OR (height >= 720 AND height < 1080))".to_string());
            }
            "sd" => {
                conditions.push("(width < 1280 AND height < 720 AND width > 0 AND height > 0)".to_string());
            }
            _ => {}
        }
    }

    // duration_range
    if let Some(dr) = filter.duration_range.as_deref().filter(|s| !s.is_empty() && *s != "all") {
        match dr {
            "under_3m" => {
                conditions.push("(duration IS NOT NULL AND duration < 180)".to_string());
            }
            "3_to_30m" => {
                conditions.push("(duration IS NOT NULL AND duration >= 180 AND duration <= 1800)".to_string());
            }
            "over_30m" => {
                conditions.push("(duration IS NOT NULL AND duration > 1800)".to_string());
            }
            _ => {}
        }
    }

    // watch_status
    if let Some(ws) = filter.watch_status.as_deref().filter(|s| !s.is_empty() && *s != "all") {
        let idx = param_values.len() + 1;
        conditions.push(format!("watch_status = ?{}", idx));
        param_values.push(ws.to_string());
    }

    // is_starred
    if let Some(starred) = filter.is_starred {
        let idx = param_values.len() + 1;
        conditions.push(format!("is_starred = ?{}", idx));
        param_values.push(if starred { "1".to_string() } else { "0".to_string() });
    }

    // color_tag
    if let Some(ct) = filter.color_tag.as_deref().filter(|s| !s.is_empty() && *s != "all") {
        let idx = param_values.len() + 1;
        conditions.push(format!("color_tag = ?{}", idx));
        param_values.push(ct.to_string());
    }

    // tag_id
    if let Some(tid) = filter.tag_id {
        let idx = param_values.len() + 1;
        conditions.push(format!("id IN (SELECT video_id FROM video_tags WHERE tag_id = ?{})", idx));
        param_values.push(tid.to_string());
    }

    // collection_id
    if let Some(cid) = filter.collection_id {
        let idx = param_values.len() + 1;
        conditions.push(format!("id IN (SELECT video_id FROM collection_items WHERE collection_id = ?{})", idx));
        param_values.push(cid.to_string());
    }

    // source_folder
    if let Some(folder) = filter.source_folder.as_deref().filter(|s| !s.is_empty()) {
        let idx = param_values.len() + 1;
        conditions.push(format!("file_path LIKE ?{}", idx));
        let folder_prefix = if folder.ends_with('/') || folder.ends_with('\\') {
            format!("{}%", folder)
        } else {
            format!("{}/%", folder)
        };
        param_values.push(folder_prefix);
    }

    let where_clause = if conditions.is_empty() {
        "".to_string()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    let order_clause = match filter.sort_by.as_deref() {
        Some("duration_desc") => "ORDER BY duration DESC NULLS LAST",
        Some("duration_asc") => "ORDER BY duration ASC NULLS LAST",
        Some("size_desc") => "ORDER BY file_size DESC",
        Some("size_asc") => "ORDER BY file_size ASC",
        Some("name_asc") => "ORDER BY file_name ASC",
        Some("name_desc") => "ORDER BY file_name DESC",
        Some("created_asc") => "ORDER BY created_at ASC",
        _ => "ORDER BY created_at DESC",
    };

    let count_sql = format!("SELECT COUNT(*) FROM videos {}", where_clause);
    let total: usize = {
        let mut stmt = conn.prepare(&count_sql).map_err(|e| e.to_string())?;
        let rows = stmt.query_row(
            rusqlite::params_from_iter(param_values.iter()),
            |r| r.get(0),
        ).map_err(|e| e.to_string())?;
        rows
    };

    let query_sql = format!(
        "SELECT id, file_path, file_name, file_hash, file_size, duration, width, height, codec, created_at, updated_at, status, thumbnail_path, is_starred, watch_status, playback_progress, color_tag, last_played_at
         FROM videos {} {} LIMIT ?{} OFFSET ?{}",
        where_clause,
        order_clause,
        param_values.len() + 1,
        param_values.len() + 2
    );

    let mut query_params = param_values;
    query_params.push((page_size as i64).to_string());
    query_params.push((offset as i64).to_string());

    let mut stmt = conn.prepare(&query_sql).map_err(|e| e.to_string())?;
    let video_iter = stmt
        .query_map(rusqlite::params_from_iter(query_params.iter()), |row| {
            let file_size: i64 = row.get(4)?;
            let is_starred_int: i64 = row.get(13).unwrap_or(0);
            let watch_status_str: String = row.get(14).unwrap_or_else(|_| "unwatched".to_string());
            let playback_prog: f64 = row.get(15).unwrap_or(0.0);
            let color_tag_str: Option<String> = row.get(16).unwrap_or(None);
            let last_played: Option<i64> = row.get(17).unwrap_or(None);

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
                is_starred: is_starred_int != 0,
                watch_status: watch_status_str,
                playback_progress: playback_prog,
                color_tag: color_tag_str,
                last_played_at: last_played,
                tags: Vec::new(),
            })
        })
        .map_err(|e| e.to_string())?;

    let mut items = Vec::new();
    for v in video_iter {
        items.push(v.map_err(|e| e.to_string())?);
    }

    // Attach tags in batch
    let video_ids: Vec<i64> = items.iter().map(|v| v.id).collect();
    if let Ok(tags_map) = crate::db::classification::get_tags_for_videos_batch(conn, &video_ids) {
        for item in &mut items {
            if let Some(tags) = tags_map.get(&item.id) {
                item.tags = tags.clone();
            }
        }
    }

    let total_pages = if total == 0 {
        1
    } else {
        (total + page_size - 1) / page_size
    };

    Ok(VideoListResponse {
        items,
        total,
        page,
        page_size,
        total_pages,
    })
}

pub fn get_video_by_id(conn: &Connection, id: i64) -> Result<Option<Video>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, file_path, file_name, file_hash, file_size, duration, width, height, codec, created_at, updated_at, status, thumbnail_path, is_starred, watch_status, playback_progress, color_tag, last_played_at
             FROM videos WHERE id = ?1",
        )
        .map_err(|e| e.to_string())?;

    let video = stmt
        .query_row([id], |row| {
            let file_size: i64 = row.get(4)?;
            let is_starred_int: i64 = row.get(13).unwrap_or(0);
            let watch_status_str: String = row.get(14).unwrap_or_else(|_| "unwatched".to_string());
            let playback_prog: f64 = row.get(15).unwrap_or(0.0);
            let color_tag_str: Option<String> = row.get(16).unwrap_or(None);
            let last_played: Option<i64> = row.get(17).unwrap_or(None);

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
                is_starred: is_starred_int != 0,
                watch_status: watch_status_str,
                playback_progress: playback_prog,
                color_tag: color_tag_str,
                last_played_at: last_played,
                tags: Vec::new(),
            })
        })
        .ok();

    let mut result = video;
    if let Some(ref mut v) = result {
        if let Ok(tags_map) = crate::db::classification::get_tags_for_videos_batch(conn, &[v.id]) {
            if let Some(tags) = tags_map.get(&v.id) {
                v.tags = tags.clone();
            }
        }
    }

    Ok(result)
}

pub fn delete_video(conn: &Connection, id: i64, delete_disk_file: bool) -> Result<bool, String> {
    let video = get_video_by_id(conn, id)?;
    if let Some(v) = video {
        conn.execute("DELETE FROM videos WHERE id = ?1", [id])
            .map_err(|e| format!("Failed to delete video from database: {}", e))?;

        let _ = conn.execute("DELETE FROM fts_index WHERE video_id = ?1", [id.to_string()]);

        if let Some(thumb) = &v.thumbnail_path {
            let _ = fs::remove_file(thumb);
        }

        if delete_disk_file {
            let path = Path::new(&v.file_path);
            if path.exists() {
                let _ = fs::remove_file(path);
            }
        }

        Ok(true)
    } else {
        Ok(false)
    }
}

/// Scans existing database records for `.ts` files and purges any records
/// that are actually TypeScript code or non-existent files.
pub fn purge_invalid_ts_records(conn: &Connection) -> Result<usize, String> {
    let mut stmt = conn
        .prepare("SELECT id, file_path FROM videos WHERE file_path LIKE '%.ts' OR file_name LIKE '%.ts'")
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)))
        .map_err(|e| e.to_string())?;

    let mut ids_to_delete = Vec::new();
    for item in rows.flatten() {
        let (id, path_str) = item;
        let path = Path::new(&path_str);
        if !path.exists() || !crate::scanner::platform::is_mpeg_ts_video(path) {
            ids_to_delete.push(id);
        }
    }

    let mut deleted_count = 0;
    for id in ids_to_delete {
        let _ = conn.execute("DELETE FROM fts_index WHERE video_id = ?1", [id.to_string()]);
        let _ = conn.execute("DELETE FROM video_tags WHERE video_id = ?1", [id]);
        if conn.execute("DELETE FROM videos WHERE id = ?1", [id]).is_ok() {
            deleted_count += 1;
        }
    }

    Ok(deleted_count)
}

