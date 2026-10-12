use crate::models::{CategoryStats, Collection, SourceFolderStat, Tag};
use rusqlite::{params, Connection, Result};
use std::collections::HashMap;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

// ==========================================
// 1. Tag Operations
// ==========================================

pub fn get_all_tags(conn: &Connection) -> Result<Vec<Tag>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT t.id, t.name, t.color, t.created_at, COUNT(vt.video_id) AS v_count
             FROM tags t
             LEFT JOIN video_tags vt ON t.id = vt.tag_id
             GROUP BY t.id
             ORDER BY t.created_at DESC",
        )
        .map_err(|e| format!("Failed to prepare get_all_tags: {}", e))?;

    let tag_iter = stmt
        .query_map([], |row| {
            let count: i64 = row.get(4)?;
            Ok(Tag {
                id: row.get(0)?,
                name: row.get(1)?,
                color: row.get(2)?,
                created_at: row.get(3)?,
                video_count: Some(count as usize),
            })
        })
        .map_err(|e| format!("Failed to query tags: {}", e))?;

    let mut list = Vec::new();
    for t in tag_iter {
        list.push(t.map_err(|e| e.to_string())?);
    }
    Ok(list)
}

pub fn create_tag(conn: &Connection, name: &str, color: Option<&str>) -> Result<Tag, String> {
    let name_trimmed = name.trim();
    if name_trimmed.is_empty() {
        return Err("Tag name cannot be empty".to_string());
    }

    let now = now_millis();
    conn.execute(
        "INSERT INTO tags (name, color, created_at) VALUES (?1, ?2, ?3)",
        params![name_trimmed, color, now],
    )
    .map_err(|e| format!("Failed to create tag: {}", e))?;

    let id = conn.last_insert_rowid();
    Ok(Tag {
        id,
        name: name_trimmed.to_string(),
        color: color.map(|s| s.to_string()),
        created_at: now,
        video_count: Some(0),
    })
}

pub fn delete_tag(conn: &Connection, tag_id: i64) -> Result<(), String> {
    conn.execute("DELETE FROM tags WHERE id = ?1", [tag_id])
        .map_err(|e| format!("Failed to delete tag: {}", e))?;
    Ok(())
}

pub fn add_tag_to_video(conn: &Connection, video_id: i64, tag_id: i64) -> Result<(), String> {
    let now = now_millis();
    conn.execute(
        "INSERT OR IGNORE INTO video_tags (video_id, tag_id, created_at) VALUES (?1, ?2, ?3)",
        params![video_id, tag_id, now],
    )
    .map_err(|e| format!("Failed to add tag to video: {}", e))?;
    Ok(())
}

pub fn remove_tag_from_video(conn: &Connection, video_id: i64, tag_id: i64) -> Result<(), String> {
    conn.execute(
        "DELETE FROM video_tags WHERE video_id = ?1 AND tag_id = ?2",
        params![video_id, tag_id],
    )
    .map_err(|e| format!("Failed to remove tag from video: {}", e))?;
    Ok(())
}

pub fn batch_add_tag_to_videos(
    conn: &Connection,
    video_ids: &[i64],
    tag_id: i64,
) -> Result<(), String> {
    let now = now_millis();
    for vid in video_ids {
        let _ = conn.execute(
            "INSERT OR IGNORE INTO video_tags (video_id, tag_id, created_at) VALUES (?1, ?2, ?3)",
            params![vid, tag_id, now],
        );
    }
    Ok(())
}

pub fn get_tags_for_videos_batch(
    conn: &Connection,
    video_ids: &[i64],
) -> Result<HashMap<i64, Vec<Tag>>, String> {
    let mut map = HashMap::new();
    if video_ids.is_empty() {
        return Ok(map);
    }

    let placeholders = video_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let sql = format!(
        "SELECT vt.video_id, t.id, t.name, t.color, t.created_at
         FROM video_tags vt
         JOIN tags t ON vt.tag_id = t.id
         WHERE vt.video_id IN ({})
         ORDER BY t.name ASC",
        placeholders
    );

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| format!("Failed to prepare get_tags_for_videos_batch: {}", e))?;

    let rows = stmt
        .query_map(rusqlite::params_from_iter(video_ids.iter()), |row| {
            let vid: i64 = row.get(0)?;
            let tag = Tag {
                id: row.get(1)?,
                name: row.get(2)?,
                color: row.get(3)?,
                created_at: row.get(4)?,
                video_count: None,
            };
            Ok((vid, tag))
        })
        .map_err(|e| format!("Query failed: {}", e))?;

    for item in rows {
        if let Ok((vid, tag)) = item {
            map.entry(vid).or_insert_with(Vec::new).push(tag);
        }
    }

    Ok(map)
}

// ==========================================
// 2. Collection Operations
// ==========================================

pub fn get_all_collections(conn: &Connection) -> Result<Vec<Collection>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT c.id, c.name, c.description, c.is_smart, c.rules_json, c.created_at, c.updated_at, COUNT(ci.video_id) AS v_count
             FROM collections c
             LEFT JOIN collection_items ci ON c.id = ci.collection_id
             GROUP BY c.id
             ORDER BY c.updated_at DESC",
        )
        .map_err(|e| format!("Failed to prepare get_all_collections: {}", e))?;

    let coll_iter = stmt
        .query_map([], |row| {
            let is_smart_int: i64 = row.get(3)?;
            let v_count: i64 = row.get(7)?;
            Ok(Collection {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                is_smart: is_smart_int != 0,
                rules_json: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
                video_count: v_count as usize,
            })
        })
        .map_err(|e| format!("Failed to query collections: {}", e))?;

    let mut list = Vec::new();
    for c in coll_iter {
        list.push(c.map_err(|e| e.to_string())?);
    }
    Ok(list)
}

pub fn create_collection(
    conn: &Connection,
    name: &str,
    description: Option<&str>,
    is_smart: bool,
    rules_json: Option<&str>,
) -> Result<Collection, String> {
    let name_trimmed = name.trim();
    if name_trimmed.is_empty() {
        return Err("Collection name cannot be empty".to_string());
    }

    let now = now_millis();
    conn.execute(
        "INSERT INTO collections (name, description, is_smart, rules_json, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            name_trimmed,
            description,
            if is_smart { 1 } else { 0 },
            rules_json,
            now,
            now
        ],
    )
    .map_err(|e| format!("Failed to create collection: {}", e))?;

    let id = conn.last_insert_rowid();
    Ok(Collection {
        id,
        name: name_trimmed.to_string(),
        description: description.map(|s| s.to_string()),
        is_smart,
        rules_json: rules_json.map(|s| s.to_string()),
        video_count: 0,
        created_at: now,
        updated_at: now,
    })
}

pub fn delete_collection(conn: &Connection, coll_id: i64) -> Result<(), String> {
    conn.execute("DELETE FROM collections WHERE id = ?1", [coll_id])
        .map_err(|e| format!("Failed to delete collection: {}", e))?;
    Ok(())
}

pub fn add_videos_to_collection(
    conn: &Connection,
    coll_id: i64,
    video_ids: &[i64],
) -> Result<(), String> {
    let now = now_millis();
    for vid in video_ids {
        let _ = conn.execute(
            "INSERT OR IGNORE INTO collection_items (collection_id, video_id, sort_order, created_at)
             VALUES (?1, ?2, 0, ?3)",
            params![coll_id, vid, now],
        );
    }
    let _ = conn.execute(
        "UPDATE collections SET updated_at = ?1 WHERE id = ?2",
        params![now, coll_id],
    );
    Ok(())
}

pub fn remove_video_from_collection(
    conn: &Connection,
    coll_id: i64,
    video_id: i64,
) -> Result<(), String> {
    conn.execute(
        "DELETE FROM collection_items WHERE collection_id = ?1 AND video_id = ?2",
        params![coll_id, video_id],
    )
    .map_err(|e| format!("Failed to remove video from collection: {}", e))?;
    Ok(())
}

// ==========================================
// 3. Video User State Updates
// ==========================================

pub fn toggle_video_star(conn: &Connection, video_id: i64) -> Result<bool, String> {
    let current_star: i64 = conn
        .query_row(
            "SELECT is_starred FROM videos WHERE id = ?1",
            [video_id],
            |r| r.get(0),
        )
        .map_err(|e| format!("Video not found: {}", e))?;

    let new_star = if current_star == 0 { 1 } else { 0 };
    conn.execute(
        "UPDATE videos SET is_starred = ?1 WHERE id = ?2",
        params![new_star, video_id],
    )
    .map_err(|e| format!("Failed to update star status: {}", e))?;

    Ok(new_star != 0)
}

pub fn update_video_watch_progress(
    conn: &Connection,
    video_id: i64,
    status: &str,
    progress: f64,
) -> Result<(), String> {
    let now = now_millis();
    conn.execute(
        "UPDATE videos SET watch_status = ?1, playback_progress = ?2, last_played_at = ?3 WHERE id = ?4",
        params![status, progress.clamp(0.0, 1.0), now, video_id],
    )
    .map_err(|e| format!("Failed to update watch progress: {}", e))?;
    Ok(())
}

pub fn set_video_color_tag(
    conn: &Connection,
    video_id: i64,
    color: Option<&str>,
) -> Result<(), String> {
    conn.execute(
        "UPDATE videos SET color_tag = ?1 WHERE id = ?2",
        params![color, video_id],
    )
    .map_err(|e| format!("Failed to set color tag: {}", e))?;
    Ok(())
}

pub fn batch_update_videos_status(
    conn: &Connection,
    video_ids: &[i64],
    watch_status: Option<&str>,
    is_starred: Option<bool>,
    color_tag: Option<&str>,
) -> Result<(), String> {
    for vid in video_ids {
        if let Some(ws) = watch_status {
            let _ = conn.execute(
                "UPDATE videos SET watch_status = ?1 WHERE id = ?2",
                params![ws, vid],
            );
        }
        if let Some(starred) = is_starred {
            let _ = conn.execute(
                "UPDATE videos SET is_starred = ?1 WHERE id = ?2",
                params![if starred { 1 } else { 0 }, vid],
            );
        }
        if let Some(ct) = color_tag {
            let color_val = if ct.is_empty() { None } else { Some(ct) };
            let _ = conn.execute(
                "UPDATE videos SET color_tag = ?1 WHERE id = ?2",
                params![color_val, vid],
            );
        }
    }
    Ok(())
}

// ==========================================
// 4. Source Folders & Category Statistics
// ==========================================

pub fn get_source_folders(conn: &Connection) -> Result<Vec<SourceFolderStat>, String> {
    let mut stmt = conn
        .prepare("SELECT file_path, file_size FROM videos")
        .map_err(|e| format!("Failed to prepare query: {}", e))?;

    let mut folder_map: HashMap<String, (usize, u64, String)> = HashMap::new();

    let rows = stmt
        .query_map([], |row| {
            let path_str: String = row.get(0)?;
            let size: i64 = row.get(1)?;
            Ok((path_str, size as u64))
        })
        .map_err(|e| format!("Query failed: {}", e))?;

    for item in rows {
        if let Ok((path_str, size)) = item {
            let p = Path::new(&path_str);
            if let Some(parent) = p.parent() {
                let parent_str = parent.to_string_lossy().to_string();
                let folder_name = parent
                    .file_name()
                    .map(|f| f.to_string_lossy().to_string())
                    .unwrap_or_else(|| parent_str.clone());

                let entry = folder_map
                    .entry(parent_str.clone())
                    .or_insert((0, 0, folder_name));
                entry.0 += 1;
                entry.1 += size;
            }
        }
    }

    let mut list: Vec<SourceFolderStat> = folder_map
        .into_iter()
        .map(
            |(folder_path, (video_count, total_size_bytes, folder_name))| SourceFolderStat {
                folder_path,
                folder_name,
                video_count,
                total_size_bytes,
            },
        )
        .collect();

    list.sort_by(|a, b| b.video_count.cmp(&a.video_count));
    Ok(list)
}

pub fn get_category_stats(conn: &Connection) -> Result<CategoryStats, String> {
    let total_videos: usize = conn
        .query_row("SELECT COUNT(*) FROM videos", [], |r| r.get(0))
        .unwrap_or(0);

    let total_size_bytes: i64 = conn
        .query_row("SELECT COALESCE(SUM(file_size), 0) FROM videos", [], |r| {
            r.get(0)
        })
        .unwrap_or(0);

    // Formats:
    // Shorts: duration <= 90 OR (width < height AND width > 0 AND height > 0)
    let shorts_count: usize = conn
        .query_row(
            "SELECT COUNT(*) FROM videos
             WHERE (duration IS NOT NULL AND duration <= 90)
                OR (width IS NOT NULL AND height IS NOT NULL AND width > 0 AND height > 0 AND width < height)",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);

    // Movies: duration >= 2700 (45 mins) AND (width >= height OR width IS NULL)
    let movies_count: usize = conn
        .query_row(
            "SELECT COUNT(*) FROM videos
             WHERE duration >= 2700
               AND (width IS NULL OR height IS NULL OR width >= height)",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);

    // Series: detected by series keyword pattern or grouped numbers in title
    let series_count: usize = conn
        .query_row(
            "SELECT COUNT(*) FROM videos
             WHERE file_name LIKE '%E0%' OR file_name LIKE '%E1%'
                OR file_name LIKE '%EP%' OR file_name LIKE '%第%集%'
                OR file_name LIKE '%第%话%' OR file_name LIKE '%S0%'",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);

    // Screen recordings: filenames containing Screen, 录屏, 截屏, OBS, Record
    let screen_count: usize = conn
        .query_row(
            "SELECT COUNT(*) FROM videos
             WHERE file_name LIKE '%Screen%' OR file_name LIKE '%录屏%'
                OR file_name LIKE '%截屏%' OR file_name LIKE '%OBS%'
                OR file_name LIKE '%Record%'",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);

    // Watch status
    let unwatched_count: usize = conn
        .query_row(
            "SELECT COUNT(*) FROM videos WHERE watch_status = 'unwatched'",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);

    let in_progress_count: usize = conn
        .query_row(
            "SELECT COUNT(*) FROM videos WHERE watch_status = 'in_progress'",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);

    let watched_count: usize = conn
        .query_row(
            "SELECT COUNT(*) FROM videos WHERE watch_status = 'watched'",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);

    let starred_count: usize = conn
        .query_row(
            "SELECT COUNT(*) FROM videos WHERE is_starred = 1",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);

    let tags = get_all_tags(conn).unwrap_or_default();
    let collections = get_all_collections(conn).unwrap_or_default();
    let source_folders = get_source_folders(conn).unwrap_or_default();

    Ok(CategoryStats {
        total_videos,
        total_size_bytes: total_size_bytes as u64,
        shorts_count,
        movies_count,
        series_count,
        screen_count,
        unwatched_count,
        in_progress_count,
        watched_count,
        starred_count,
        tags,
        collections,
        source_folders,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migrations::apply_all;
    use crate::db::videos::insert_or_update_video;

    fn setup_test_db() -> Connection {
        let conn = Connection::open_in_memory().expect("open memory db");
        apply_all(&conn).expect("apply migrations");
        conn
    }

    #[test]
    fn test_tags_crud_and_video_association() {
        let conn = setup_test_db();

        let tag = create_tag(&conn, "Rust 教程", Some("#3b82f6")).expect("create tag");
        assert_eq!(tag.name, "Rust 教程");
        assert_eq!(tag.color.as_deref(), Some("#3b82f6"));

        // Insert video
        let v_id = insert_or_update_video(
            &conn,
            "/path/to/rust.mp4",
            "rust.mp4",
            "hash123",
            1024,
            Some(60.0),
            Some(1920),
            Some(1080),
            Some("H264"),
            None,
            1000,
        )
        .expect("insert video");

        add_tag_to_video(&conn, v_id, tag.id).expect("add tag to video");

        let batch_tags = get_tags_for_videos_batch(&conn, &[v_id]).expect("batch tags");
        assert_eq!(batch_tags.get(&v_id).unwrap().len(), 1);
        assert_eq!(batch_tags.get(&v_id).unwrap()[0].name, "Rust 教程");

        let all_tags = get_all_tags(&conn).expect("get all tags");
        assert_eq!(all_tags[0].video_count, Some(1));

        remove_tag_from_video(&conn, v_id, tag.id).expect("remove tag");
        let batch_tags_after = get_tags_for_videos_batch(&conn, &[v_id]).expect("batch tags after");
        assert!(batch_tags_after.get(&v_id).is_none());

        delete_tag(&conn, tag.id).expect("delete tag");
        assert_eq!(get_all_tags(&conn).expect("empty tags").len(), 0);
    }

    #[test]
    fn test_collections_crud() {
        let conn = setup_test_db();
        let coll = create_collection(&conn, "我的精选", Some("测试说明"), false, None)
            .expect("create collection");
        assert_eq!(coll.name, "我的精选");

        let v_id = insert_or_update_video(
            &conn,
            "/videos/demo.mp4",
            "demo.mp4",
            "h2",
            2048,
            Some(120.0),
            Some(1280),
            Some(720),
            Some("H264"),
            None,
            1000,
        )
        .expect("insert video");

        add_videos_to_collection(&conn, coll.id, &[v_id]).expect("add video to collection");
        let list = get_all_collections(&conn).expect("get all collections");
        assert_eq!(list[0].video_count, 1);

        remove_video_from_collection(&conn, coll.id, v_id).expect("remove video from coll");
        delete_collection(&conn, coll.id).expect("delete coll");
    }

    #[test]
    fn test_video_states_and_category_stats() {
        let conn = setup_test_db();

        // 1. Insert a short vertical video
        let v_short = insert_or_update_video(
            &conn,
            "/clips/short_reel.mp4",
            "short_reel.mp4",
            "hs1",
            5000,
            Some(45.0),
            Some(1080),
            Some(1920), // vertical
            Some("H264"),
            None,
            1000,
        )
        .expect("insert short");

        // 2. Insert a movie
        let v_movie = insert_or_update_video(
            &conn,
            "/movies/inception.mp4",
            "inception.mp4",
            "hm1",
            5000000,
            Some(3600.0), // 1 hour
            Some(3840),
            Some(2160),
            Some("HEVC"),
            None,
            1000,
        )
        .expect("insert movie");

        // Toggle star
        let starred = toggle_video_star(&conn, v_short).expect("star short");
        assert!(starred);

        // Update progress
        update_video_watch_progress(&conn, v_movie, "in_progress", 0.35).expect("update progress");
        set_video_color_tag(&conn, v_short, Some("red")).expect("set color");

        let stats = get_category_stats(&conn).expect("get stats");
        assert_eq!(stats.total_videos, 2);
        assert_eq!(stats.shorts_count, 1);
        assert_eq!(stats.movies_count, 1);
        assert_eq!(stats.starred_count, 1);
        assert_eq!(stats.in_progress_count, 1);
        assert_eq!(stats.unwatched_count, 1);
    }
}
