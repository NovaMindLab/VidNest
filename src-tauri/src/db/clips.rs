use crate::models::{ClippedVideo, ClippedVideoFilter, MasterVideoClipsGroup};
use rusqlite::{params, Connection, Result};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Inserts a new clipped video record or updates an existing one by file_path.
pub fn insert_or_update_clipped_video(
    conn: &Connection,
    video_id: Option<i64>,
    source_video_title: Option<&str>,
    source_video_path: &str,
    file_path: &str,
    file_name: &str,
    title: &str,
    start_ms: i64,
    end_ms: i64,
    duration: Option<f64>,
    file_size: u64,
    is_merged: bool,
    created_at: i64,
) -> Result<i64, String> {
    let existing_id: Option<i64> = conn
        .query_row(
            "SELECT id FROM clipped_videos WHERE file_path = ?1",
            [file_path],
            |row| row.get(0),
        )
        .ok();

    if let Some(id) = existing_id {
        conn.execute(
            "UPDATE clipped_videos SET
                video_id = ?1,
                source_video_title = ?2,
                source_video_path = ?3,
                file_name = ?4,
                title = ?5,
                start_ms = ?6,
                end_ms = ?7,
                duration = ?8,
                file_size = ?9,
                is_merged = ?10
             WHERE id = ?11",
            params![
                video_id,
                source_video_title,
                source_video_path,
                file_name,
                title,
                start_ms,
                end_ms,
                duration,
                file_size as i64,
                if is_merged { 1 } else { 0 },
                id,
            ],
        )
        .map_err(|e| format!("Failed to update clipped video: {}", e))?;
        Ok(id)
    } else {
        conn.execute(
            "INSERT INTO clipped_videos (
                video_id, source_video_title, source_video_path, file_path, file_name, title,
                start_ms, end_ms, duration, file_size, is_merged, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                video_id,
                source_video_title,
                source_video_path,
                file_path,
                file_name,
                title,
                start_ms,
                end_ms,
                duration,
                file_size as i64,
                if is_merged { 1 } else { 0 },
                created_at,
            ],
        )
        .map_err(|e| format!("Failed to insert clipped video: {}", e))?;
        Ok(conn.last_insert_rowid())
    }
}

/// Lists all clipped video records with optional search, source filter, and sorting.
pub fn list_clipped_videos(
    conn: &Connection,
    filter: &ClippedVideoFilter,
) -> Result<Vec<ClippedVideo>, String> {
    let mut sql = String::from(
        "SELECT id, video_id, source_video_title, source_video_path, file_path, file_name, title,
                start_ms, end_ms, duration, file_size, is_merged, created_at
         FROM clipped_videos WHERE 1=1",
    );
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(ref search) = filter.search {
        let q = search.trim();
        if !q.is_empty() {
            let pattern = format!("%{}%", q);
            sql.push_str(" AND (title LIKE ? OR file_name LIKE ? OR source_video_title LIKE ? OR file_path LIKE ?)");
            params.push(Box::new(pattern.clone()));
            params.push(Box::new(pattern.clone()));
            params.push(Box::new(pattern.clone()));
            params.push(Box::new(pattern));
        }
    }

    if let Some(ref src) = filter.source_video_path {
        let s = src.trim();
        if !s.is_empty() {
            sql.push_str(" AND source_video_path = ?");
            params.push(Box::new(s.to_string()));
        }
    }

    if let Some(merged_only) = filter.is_merged_only {
        if merged_only {
            sql.push_str(" AND is_merged = 1");
        }
    }

    match filter.sort_by.as_deref().unwrap_or("created_desc") {
        "created_asc" => sql.push_str(" ORDER BY created_at ASC"),
        "duration_desc" => sql.push_str(" ORDER BY duration DESC"),
        "title_asc" => sql.push_str(" ORDER BY title ASC"),
        _ => sql.push_str(" ORDER BY created_at DESC"),
    }

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| format!("Failed to prepare list_clipped_videos: {}", e))?;

    let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();

    let rows = stmt
        .query_map(param_refs.as_slice(), |row| {
            let is_merged_int: i32 = row.get(11)?;
            let file_size_i64: i64 = row.get(10)?;
            Ok(ClippedVideo {
                id: row.get(0)?,
                video_id: row.get(1)?,
                source_video_title: row.get(2)?,
                source_video_path: row.get(3)?,
                file_path: row.get(4)?,
                file_name: row.get(5)?,
                title: row.get(6)?,
                start_ms: row.get(7)?,
                end_ms: row.get(8)?,
                duration: row.get(9)?,
                file_size: file_size_i64.max(0) as u64,
                is_merged: is_merged_int != 0,
                created_at: row.get(12)?,
            })
        })
        .map_err(|e| format!("Failed to execute list_clipped_videos: {}", e))?;

    let mut results = Vec::new();
    for r in rows {
        results.push(r.map_err(|e| format!("Row mapping error: {}", e))?);
    }

    Ok(results)
}

/// Lists clipped videos grouped by master video (source_video_path)
pub fn list_clipped_videos_grouped(
    conn: &Connection,
    filter: &ClippedVideoFilter,
) -> Result<Vec<MasterVideoClipsGroup>, String> {
    let all_clips = list_clipped_videos(conn, filter)?;

    // Preserve order of appearance while grouping
    let mut groups_order: Vec<String> = Vec::new();
    let mut groups_map: HashMap<String, MasterVideoClipsGroup> = HashMap::new();

    for clip in all_clips {
        let key = clip.source_video_path.clone();
        if !groups_map.contains_key(&key) {
            groups_order.push(key.clone());
            let title = clip
                .source_video_title
                .clone()
                .unwrap_or_else(|| {
                    Path::new(&clip.source_video_path)
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("未知主视频")
                        .to_string()
                });

            groups_map.insert(
                key.clone(),
                MasterVideoClipsGroup {
                    source_video_path: clip.source_video_path.clone(),
                    source_video_title: title,
                    video_id: clip.video_id,
                    total_clips: 0,
                    total_duration: 0.0,
                    total_size: 0,
                    latest_created_at: clip.created_at,
                    clips: Vec::new(),
                },
            );
        }

        if let Some(group) = groups_map.get_mut(&key) {
            group.total_clips += 1;
            group.total_duration += clip.duration.unwrap_or(0.0);
            group.total_size += clip.file_size;
            if clip.created_at > group.latest_created_at {
                group.latest_created_at = clip.created_at;
            }
            group.clips.push(clip);
        }
    }

    // Sort clips within each group by start_ms ASC
    for group in groups_map.values_mut() {
        group.clips.sort_by_key(|c| c.start_ms);
    }

    // Sort groups by latest_created_at DESC
    let mut result: Vec<MasterVideoClipsGroup> = groups_order
        .into_iter()
        .filter_map(|k| groups_map.remove(&k))
        .collect();

    result.sort_by(|a, b| b.latest_created_at.cmp(&a.latest_created_at));

    Ok(result)
}

/// Deletes a clipped video record, optionally removing the disk file.
pub fn delete_clipped_video(
    conn: &Connection,
    id: i64,
    delete_disk_file: bool,
) -> Result<(), String> {
    let file_path: Option<String> = conn
        .query_row(
            "SELECT file_path FROM clipped_videos WHERE id = ?1",
            [id],
            |row| row.get(0),
        )
        .ok();

    conn.execute("DELETE FROM clipped_videos WHERE id = ?1", [id])
        .map_err(|e| format!("Failed to delete clipped video from database: {}", e))?;

    if delete_disk_file {
        if let Some(path_str) = file_path {
            let path = Path::new(&path_str);
            if path.exists() {
                let _ = fs::remove_file(path);
            }
        }
    }

    Ok(())
}

/// Deletes all clipped videos belonging to a master video, optionally deleting disk files.
pub fn batch_delete_master_clips(
    conn: &Connection,
    source_video_path: &str,
    delete_disk_file: bool,
) -> Result<usize, String> {
    let mut stmt = conn
        .prepare("SELECT file_path FROM clipped_videos WHERE source_video_path = ?1")
        .map_err(|e| format!("Failed to query master clips: {}", e))?;

    let file_paths: Vec<String> = stmt
        .query_map([source_video_path], |row| row.get(0))
        .map_err(|e| format!("Failed to fetch file paths: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

    let deleted_count = conn
        .execute(
            "DELETE FROM clipped_videos WHERE source_video_path = ?1",
            [source_video_path],
        )
        .map_err(|e| format!("Failed to delete master clips from database: {}", e))?;

    if delete_disk_file {
        for path_str in file_paths {
            let path = Path::new(&path_str);
            if path.exists() {
                let _ = fs::remove_file(path);
            }
        }
    }

    Ok(deleted_count)
}

/// Fetches a single clipped video record by ID.
pub fn get_clipped_video_by_id(
    conn: &Connection,
    id: i64,
) -> Result<Option<ClippedVideo>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, video_id, source_video_title, source_video_path, file_path, file_name, title,
                    start_ms, end_ms, duration, file_size, is_merged, created_at
             FROM clipped_videos WHERE id = ?1",
        )
        .map_err(|e| format!("Failed to prepare get_clipped_video_by_id: {}", e))?;

    let mut rows = stmt
        .query_map([id], |row| {
            let is_merged_int: i32 = row.get(11)?;
            let file_size_i64: i64 = row.get(10)?;
            Ok(ClippedVideo {
                id: row.get(0)?,
                video_id: row.get(1)?,
                source_video_title: row.get(2)?,
                source_video_path: row.get(3)?,
                file_path: row.get(4)?,
                file_name: row.get(5)?,
                title: row.get(6)?,
                start_ms: row.get(7)?,
                end_ms: row.get(8)?,
                duration: row.get(9)?,
                file_size: file_size_i64.max(0) as u64,
                is_merged: is_merged_int != 0,
                created_at: row.get(12)?,
            })
        })
        .map_err(|e| format!("Failed to execute get_clipped_video_by_id: {}", e))?;

    if let Some(res) = rows.next() {
        res.map(Some).map_err(|e| e.to_string())
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migrations::apply_all;

    #[test]
    fn test_clipped_video_crud_and_grouping() {
        let conn = Connection::open_in_memory().unwrap();
        apply_all(&conn).unwrap();

        // Seed master videos
        conn.execute(
            "INSERT INTO videos (id, file_path, file_name, file_hash, file_size, created_at, updated_at)
             VALUES (101, '/path/to/movie_a.mp4', 'movie_a.mp4', 'hash1', 1000, 1000, 1000)",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO videos (id, file_path, file_name, file_hash, file_size, created_at, updated_at)
             VALUES (102, '/path/to/tutorial_b.mp4', 'tutorial_b.mp4', 'hash2', 2000, 1000, 1000)",
            [],
        )
        .unwrap();

        // 1. Insert 2 clips for video A
        let id1 = insert_or_update_clipped_video(
            &conn,
            Some(101),
            Some("movie_a.mp4"),
            "/path/to/movie_a.mp4",
            "/path/to/movie_a_clip_01.mp4",
            "movie_a_clip_01.mp4",
            "精彩开局",
            0,
            60000,
            Some(60.0),
            15000000,
            false,
            1700000100,
        )
        .unwrap();

        let id2 = insert_or_update_clipped_video(
            &conn,
            Some(101),
            Some("movie_a.mp4"),
            "/path/to/movie_a.mp4",
            "/path/to/movie_a_clip_02.mp4",
            "movie_a_clip_02.mp4",
            "巅峰决战",
            120000,
            240000,
            Some(120.0),
            32000000,
            false,
            1700000200,
        )
        .unwrap();

        // 2. Insert 1 clip for video B
        let _id3 = insert_or_update_clipped_video(
            &conn,
            Some(102),
            Some("tutorial_b.mp4"),
            "/path/to/tutorial_b.mp4",
            "/path/to/tutorial_b_clip_01.mp4",
            "tutorial_b_clip_01.mp4",
            "核心重点讲解",
            30000,
            90000,
            Some(60.0),
            12000000,
            false,
            1700000300,
        )
        .unwrap();

        assert!(id1 > 0);
        assert!(id2 > 0);

        // 3. Test list_clipped_videos
        let filter = ClippedVideoFilter::default();
        let all_clips = list_clipped_videos(&conn, &filter).unwrap();
        assert_eq!(all_clips.len(), 3);

        // 4. Test list_clipped_videos_grouped
        let groups = list_clipped_videos_grouped(&conn, &filter).unwrap();
        assert_eq!(groups.len(), 2);

        // Group B was created later, so it appears first in latest_created_at
        assert_eq!(groups[0].source_video_path, "/path/to/tutorial_b.mp4");
        assert_eq!(groups[0].total_clips, 1);

        // Group A has 2 clips
        assert_eq!(groups[1].source_video_path, "/path/to/movie_a.mp4");
        assert_eq!(groups[1].total_clips, 2);
        assert_eq!(groups[1].total_size, 47000000);
        assert_eq!(groups[1].total_duration, 180.0);

        // 5. Test search filter
        let search_filter = ClippedVideoFilter {
            search: Some("巅峰".to_string()),
            ..Default::default()
        };
        let search_groups = list_clipped_videos_grouped(&conn, &search_filter).unwrap();
        assert_eq!(search_groups.len(), 1);
        assert_eq!(search_groups[0].clips[0].title, "巅峰决战");

        // 6. Test delete_clipped_video
        delete_clipped_video(&conn, id1, false).unwrap();
        let after_delete = list_clipped_videos(&conn, &filter).unwrap();
        assert_eq!(after_delete.len(), 2);

        // 7. Test batch_delete_master_clips
        let deleted_count = batch_delete_master_clips(&conn, "/path/to/movie_a.mp4", false).unwrap();
        assert_eq!(deleted_count, 1);
        let final_clips = list_clipped_videos(&conn, &filter).unwrap();
        assert_eq!(final_clips.len(), 1);
    }
}
