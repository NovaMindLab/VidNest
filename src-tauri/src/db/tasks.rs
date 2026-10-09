use crate::models::{DownloadTask, VideoSource};
use rusqlite::{params, Connection, Result};

pub fn insert_download_task(conn: &Connection, task: &DownloadTask) -> Result<(), String> {
    conn.execute(
        "INSERT INTO download_tasks (
            id, url, title, author, platform, format_id,
            status, progress, downloaded_bytes, total_bytes,
            speed, speed_str, eta, file_path, error_log,
            created_at, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
        params![
            task.id,
            task.url,
            task.title,
            task.author,
            task.platform,
            task.format_id,
            task.status,
            task.progress,
            task.downloaded_bytes as i64,
            task.total_bytes as i64,
            task.speed,
            task.speed_str,
            task.eta,
            task.file_path,
            task.error_log,
            task.created_at,
            task.updated_at,
        ],
    )
    .map_err(|e| format!("Failed to insert download task {}: {}", task.id, e))?;

    Ok(())
}

pub fn update_download_task(conn: &Connection, task: &DownloadTask) -> Result<(), String> {
    conn.execute(
        "UPDATE download_tasks SET
            status = ?1,
            progress = ?2,
            downloaded_bytes = ?3,
            total_bytes = ?4,
            speed = ?5,
            speed_str = ?6,
            eta = ?7,
            file_path = ?8,
            error_log = ?9,
            title = COALESCE(?10, title),
            author = COALESCE(?11, author),
            platform = COALESCE(?12, platform),
            updated_at = ?13
         WHERE id = ?14",
        params![
            task.status,
            task.progress,
            task.downloaded_bytes as i64,
            task.total_bytes as i64,
            task.speed,
            task.speed_str,
            task.eta,
            task.file_path,
            task.error_log,
            task.title,
            task.author,
            task.platform,
            task.updated_at,
            task.id,
        ],
    )
    .map_err(|e| format!("Failed to update download task {}: {}", task.id, e))?;

    Ok(())
}

pub fn load_all_download_tasks(conn: &Connection) -> Result<Vec<DownloadTask>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, url, title, author, platform, format_id,
                    status, progress, downloaded_bytes, total_bytes,
                    speed, speed_str, eta, file_path, error_log,
                    created_at, updated_at
             FROM download_tasks
             ORDER BY created_at DESC",
        )
        .map_err(|e| e.to_string())?;

    let iter = stmt
        .query_map([], |row| {
            let dl_bytes: i64 = row.get(8).unwrap_or(0);
            let tot_bytes: i64 = row.get(9).unwrap_or(0);
            Ok(DownloadTask {
                id: row.get(0)?,
                url: row.get(1)?,
                title: row.get(2)?,
                author: row.get(3)?,
                platform: row.get(4)?,
                format_id: row.get(5)?,
                status: row.get(6)?,
                progress: row.get(7)?,
                downloaded_bytes: dl_bytes.max(0) as u64,
                total_bytes: tot_bytes.max(0) as u64,
                speed: row.get(10)?,
                speed_str: row.get(11)?,
                eta: row.get(12)?,
                file_path: row.get(13)?,
                error_log: row.get(14)?,
                created_at: row.get(15)?,
                updated_at: row.get(16)?,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut tasks = Vec::new();
    for t in iter {
        tasks.push(t.map_err(|e| e.to_string())?);
    }

    Ok(tasks)
}

pub fn delete_download_task(conn: &Connection, task_id: &str) -> Result<bool, String> {
    let count = conn
        .execute("DELETE FROM download_tasks WHERE id = ?1", [task_id])
        .map_err(|e| format!("Failed to delete download task: {}", e))?;

    Ok(count > 0)
}

pub fn clear_completed_download_tasks(conn: &Connection) -> Result<usize, String> {
    let count = conn
        .execute(
            "DELETE FROM download_tasks WHERE status IN ('completed', 'cancelled')",
            [],
        )
        .map_err(|e| format!("Failed to clear completed download tasks: {}", e))?;

    Ok(count)
}

pub fn insert_video_source(
    conn: &Connection,
    video_id: i64,
    original_url: Option<&str>,
    platform: Option<&str>,
    author: Option<&str>,
    download_task_id: Option<&str>,
    downloaded_at: i64,
) -> Result<i64, String> {
    conn.execute(
        "INSERT INTO video_sources (video_id, original_url, platform, author, download_task_id, downloaded_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            video_id,
            original_url,
            platform,
            author,
            download_task_id,
            downloaded_at,
        ],
    )
    .map_err(|e| format!("Failed to insert video_sources record: {}", e))?;

    Ok(conn.last_insert_rowid())
}

pub fn get_video_source_by_video_id(
    conn: &Connection,
    video_id: i64,
) -> Result<Option<VideoSource>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, video_id, original_url, platform, author, download_task_id, downloaded_at
             FROM video_sources WHERE video_id = ?1",
        )
        .map_err(|e| e.to_string())?;

    let source = stmt
        .query_row([video_id], |row| {
            Ok(VideoSource {
                id: row.get(0)?,
                video_id: row.get(1)?,
                original_url: row.get(2)?,
                platform: row.get(3)?,
                author: row.get(4)?,
                download_task_id: row.get(5)?,
                downloaded_at: row.get(6)?,
            })
        })
        .ok();

    Ok(source)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_db_operations() {
        let conn = rusqlite::Connection::open_in_memory().expect("open db");
        crate::db::migrations::apply_all(&conn).expect("apply migrations");

        let task = DownloadTask {
            id: "task_test_1".to_string(),
            url: "https://example.com/video.mp4".to_string(),
            title: Some("Test Video".to_string()),
            author: Some("Author 1".to_string()),
            platform: Some("youtube".to_string()),
            format_id: Some("1080p".to_string()),
            status: "downloading".to_string(),
            progress: 45.0,
            downloaded_bytes: 4500,
            total_bytes: 10000,
            speed: 1024.0,
            speed_str: Some("1KiB/s".to_string()),
            eta: Some("00:05".to_string()),
            file_path: None,
            error_log: None,
            created_at: 1000,
            updated_at: 1005,
        };

        insert_download_task(&conn, &task).expect("insert task");

        let all = load_all_download_tasks(&conn).expect("load tasks");
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id, "task_test_1");
        assert_eq!(all[0].progress, 45.0);

        let mut updated = task.clone();
        updated.status = "completed".to_string();
        updated.progress = 100.0;
        updated.file_path = Some("/path/to/video.mp4".to_string());
        update_download_task(&conn, &updated).expect("update task");

        let all2 = load_all_download_tasks(&conn).expect("load tasks");
        assert_eq!(all2[0].status, "completed");
        assert_eq!(all2[0].progress, 100.0);

        let cleared = clear_completed_download_tasks(&conn).expect("clear tasks");
        assert_eq!(cleared, 1);
        let all3 = load_all_download_tasks(&conn).expect("load tasks");
        assert_eq!(all3.len(), 0);
    }
}
