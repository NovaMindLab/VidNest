use crate::models::{ExtractedAudio, ExtractedAudioFilter};
use rusqlite::{params, Connection, Result};
use std::fs;
use std::path::Path;

/// Inserts a new extracted audio record or updates an existing one by file_path.
pub fn insert_or_update_extracted_audio(
    conn: &Connection,
    video_id: Option<i64>,
    source_video_title: Option<&str>,
    source_video_path: Option<&str>,
    file_path: &str,
    file_name: &str,
    file_size: u64,
    duration: Option<f64>,
    format: &str,
    quality_bitrate: Option<&str>,
    created_at: i64,
) -> Result<i64, String> {
    let existing_id: Option<i64> = conn
        .query_row(
            "SELECT id FROM extracted_audios WHERE file_path = ?1",
            [file_path],
            |row| row.get(0),
        )
        .ok();

    if let Some(id) = existing_id {
        conn.execute(
            "UPDATE extracted_audios SET
                video_id = ?1,
                source_video_title = ?2,
                source_video_path = ?3,
                file_name = ?4,
                file_size = ?5,
                duration = ?6,
                format = ?7,
                quality_bitrate = ?8
             WHERE id = ?9",
            params![
                video_id,
                source_video_title,
                source_video_path,
                file_name,
                file_size as i64,
                duration,
                format,
                quality_bitrate,
                id,
            ],
        )
        .map_err(|e| format!("Failed to update extracted audio: {}", e))?;
        Ok(id)
    } else {
        conn.execute(
            "INSERT INTO extracted_audios (
                video_id, source_video_title, source_video_path, file_path, file_name, file_size, duration, format, quality_bitrate, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                video_id,
                source_video_title,
                source_video_path,
                file_path,
                file_name,
                file_size as i64,
                duration,
                format,
                quality_bitrate,
                created_at,
            ],
        )
        .map_err(|e| format!("Failed to insert extracted audio: {}", e))?;
        Ok(conn.last_insert_rowid())
    }
}

/// Lists all extracted audio records with optional search, format filter, and sorting.
pub fn list_extracted_audios(
    conn: &Connection,
    filter: &ExtractedAudioFilter,
) -> Result<Vec<ExtractedAudio>, String> {
    let mut sql = String::from(
        "SELECT id, video_id, source_video_title, source_video_path, file_path, file_name, file_size, duration, format, quality_bitrate, created_at
         FROM extracted_audios WHERE 1=1"
    );

    let mut param_values: Vec<rusqlite::types::Value> = Vec::new();

    if let Some(search) = &filter.search {
        let trimmed = search.trim();
        if !trimmed.is_empty() {
            sql.push_str(" AND (file_name LIKE ? OR source_video_title LIKE ? OR file_path LIKE ?)");
            let pattern = format!("%{}%", trimmed);
            param_values.push(pattern.clone().into());
            param_values.push(pattern.clone().into());
            param_values.push(pattern.into());
        }
    }

    if let Some(fmt) = &filter.format {
        let trimmed = fmt.trim().to_lowercase();
        if !trimmed.is_empty() && trimmed != "all" {
            sql.push_str(" AND LOWER(format) = ?");
            param_values.push(trimmed.into());
        }
    }

    let order_clause = match filter.sort_by.as_deref() {
        Some("duration_desc") => " ORDER BY duration DESC NULLS LAST",
        Some("size_desc") => " ORDER BY file_size DESC",
        Some("name_asc") => " ORDER BY file_name COLLATE NOCASE ASC",
        _ => " ORDER BY created_at DESC",
    };
    sql.push_str(order_clause);

    let mut stmt = conn.prepare(&sql).map_err(|e| format!("Failed to prepare query: {}", e))?;
    let params_refs: Vec<&dyn rusqlite::ToSql> = param_values.iter().map(|v| v as &dyn rusqlite::ToSql).collect();

    let rows = stmt
        .query_map(&params_refs[..], |row| {
            let size_i64: i64 = row.get(6)?;
            Ok(ExtractedAudio {
                id: row.get(0)?,
                video_id: row.get(1)?,
                source_video_title: row.get(2)?,
                source_video_path: row.get(3)?,
                file_path: row.get(4)?,
                file_name: row.get(5)?,
                file_size: size_i64.max(0) as u64,
                duration: row.get(7)?,
                format: row.get(8)?,
                quality_bitrate: row.get(9)?,
                created_at: row.get(10)?,
            })
        })
        .map_err(|e| format!("Failed to query extracted audios: {}", e))?;

    let mut result = Vec::new();
    for r in rows {
        result.push(r.map_err(|e| format!("Failed to read audio row: {}", e))?);
    }

    Ok(result)
}

/// Deletes an extracted audio record, optionally deleting the physical disk file as well.
pub fn delete_extracted_audio(
    conn: &Connection,
    id: i64,
    delete_disk_file: bool,
) -> Result<(), String> {
    let file_path: Option<String> = conn
        .query_row(
            "SELECT file_path FROM extracted_audios WHERE id = ?1",
            [id],
            |row| row.get(0),
        )
        .ok();

    conn.execute("DELETE FROM extracted_audios WHERE id = ?1", [id])
        .map_err(|e| format!("Failed to delete audio from database: {}", e))?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migrations::apply_all;

    #[test]
    fn test_extracted_audio_crud() {
        let conn = Connection::open_in_memory().unwrap();
        apply_all(&conn).unwrap();

        let id = insert_or_update_extracted_audio(
            &conn,
            None,
            Some("test_video.mp4"),
            Some("/path/to/test_video.mp4"),
            "/path/to/test_video.mp3",
            "test_video.mp3",
            1024000,
            Some(120.5),
            "mp3",
            Some("320k"),
            1700000000,
        )
        .unwrap();

        assert!(id > 0);

        let list = list_extracted_audios(&conn, &ExtractedAudioFilter::default()).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].file_name, "test_video.mp3");
        assert_eq!(list[0].format, "mp3");
        assert_eq!(list[0].file_size, 1024000);

        // Filter by format
        let list_m4a = list_extracted_audios(
            &conn,
            &ExtractedAudioFilter {
                format: Some("m4a".to_string()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(list_m4a.len(), 0);

        // Filter by search
        let list_search = list_extracted_audios(
            &conn,
            &ExtractedAudioFilter {
                search: Some("test_video".to_string()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(list_search.len(), 1);

        // Delete without disk file
        delete_extracted_audio(&conn, id, false).unwrap();
        let list_after = list_extracted_audios(&conn, &ExtractedAudioFilter::default()).unwrap();
        assert_eq!(list_after.len(), 0);
    }
}
