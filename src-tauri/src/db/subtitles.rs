use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SubtitleSegment {
    pub id: Option<i64>,
    pub video_id: i64,
    pub language: Option<String>,
    pub format: Option<String>,
    pub start_time_ms: i64,
    pub end_time_ms: i64,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtitleSnippetMatch {
    pub video_id: i64,
    pub start_time_ms: i64,
    pub end_time_ms: i64,
    pub content: String,
}

/// Saves subtitles for a video, replacing any existing ones, and updates the FTS5 search index.
pub fn save_video_subtitles(
    conn: &mut Connection,
    video_id: i64,
    language: Option<&str>,
    format_type: Option<&str>,
    segments: &[SubtitleSegment],
) -> Result<usize, String> {
    let tx = conn
        .transaction()
        .map_err(|e| format!("Failed to begin transaction: {}", e))?;

    // 1. Remove existing subtitles for this video
    tx.execute(
        "DELETE FROM subtitles WHERE video_id = ?1",
        params![video_id],
    )
    .map_err(|e| format!("Failed to delete old subtitles: {}", e))?;

    // 2. Insert new segments
    let mut inserted_count = 0;
    let mut full_subtitles_text = String::new();

    {
        let mut stmt = tx
            .prepare(
                "INSERT INTO subtitles (video_id, language, format, start_time_ms, end_time_ms, content)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            )
            .map_err(|e| format!("Failed to prepare subtitle insert: {}", e))?;

        for seg in segments {
            let content_trimmed = seg.content.trim();
            if content_trimmed.is_empty() {
                continue;
            }

            stmt.execute(params![
                video_id,
                language.or(seg.language.as_deref()),
                format_type.or(seg.format.as_deref()),
                seg.start_time_ms,
                seg.end_time_ms,
                content_trimmed,
            ])
            .map_err(|e| format!("Failed to insert subtitle segment: {}", e))?;

            inserted_count += 1;
            if !full_subtitles_text.is_empty() {
                full_subtitles_text.push(' ');
            }
            full_subtitles_text.push_str(content_trimmed);
        }
    }

    // 3. Update FTS5 index: preserve title, update subtitles field
    let (title, summary, tags): (String, String, String) = tx
        .query_row(
            "SELECT COALESCE(title, ''), COALESCE(summary, ''), COALESCE(tags, '')
             FROM fts_index WHERE video_id = ?1",
            params![video_id.to_string()],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .or_else(|_| {
            // Fallback: look up file_name from videos table
            tx.query_row(
                "SELECT file_name FROM videos WHERE id = ?1",
                params![video_id],
                |row| {
                    let name: String = row.get(0)?;
                    Ok((name, String::new(), String::new()))
                },
            )
        })
        .unwrap_or_else(|_| (String::new(), String::new(), String::new()));

    let _ = tx.execute(
        "DELETE FROM fts_index WHERE video_id = ?1",
        params![video_id.to_string()],
    );

    tx.execute(
        "INSERT INTO fts_index (video_id, title, subtitles, summary, tags) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            video_id.to_string(),
            title,
            full_subtitles_text,
            summary,
            tags
        ],
    )
    .map_err(|e| format!("Failed to update FTS5 subtitles index: {}", e))?;

    tx.commit()
        .map_err(|e| format!("Failed to commit subtitles: {}", e))?;

    Ok(inserted_count)
}

/// Retrieves all subtitle segments for a video ordered by start time.
pub fn get_video_subtitles(
    conn: &Connection,
    video_id: i64,
) -> Result<Vec<SubtitleSegment>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, video_id, language, format, start_time_ms, end_time_ms, content
             FROM subtitles
             WHERE video_id = ?1
             ORDER BY start_time_ms ASC",
        )
        .map_err(|e| format!("Failed to prepare subtitles query: {}", e))?;

    let rows = stmt
        .query_map(params![video_id], |row| {
            Ok(SubtitleSegment {
                id: Some(row.get(0)?),
                video_id: row.get(1)?,
                language: row.get(2)?,
                format: row.get(3)?,
                start_time_ms: row.get(4)?,
                end_time_ms: row.get(5)?,
                content: row.get(6)?,
            })
        })
        .map_err(|e| format!("Failed to query subtitles: {}", e))?;

    let mut result = Vec::new();
    for r in rows {
        if let Ok(seg) = r {
            result.push(seg);
        }
    }

    Ok(result)
}

/// Deletes all subtitles for a video and clears the FTS5 subtitles column.
pub fn delete_video_subtitles(conn: &mut Connection, video_id: i64) -> Result<(), String> {
    let tx = conn
        .transaction()
        .map_err(|e| format!("Failed to begin transaction: {}", e))?;

    tx.execute(
        "DELETE FROM subtitles WHERE video_id = ?1",
        params![video_id],
    )
    .map_err(|e| format!("Failed to delete subtitles: {}", e))?;

    // Update FTS5 index to remove subtitles
    let (title, summary, tags): (String, String, String) = tx
        .query_row(
            "SELECT COALESCE(title, ''), COALESCE(summary, ''), COALESCE(tags, '')
             FROM fts_index WHERE video_id = ?1",
            params![video_id.to_string()],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap_or_else(|_| (String::new(), String::new(), String::new()));

    let _ = tx.execute(
        "DELETE FROM fts_index WHERE video_id = ?1",
        params![video_id.to_string()],
    );

    if !title.is_empty() {
        let _ = tx.execute(
            "INSERT INTO fts_index (video_id, title, subtitles, summary, tags) VALUES (?1, ?2, '', ?3, ?4)",
            params![video_id.to_string(), title, summary, tags],
        );
    }

    tx.commit()
        .map_err(|e| format!("Failed to commit delete subtitles: {}", e))?;

    Ok(())
}

/// Searches subtitles matching a query and returns snippet matches.
pub fn search_subtitles_snippets(
    conn: &Connection,
    query: &str,
    limit: usize,
) -> Result<Vec<SubtitleSnippetMatch>, String> {
    let pattern = format!("%{}%", query.trim());
    let mut stmt = conn
        .prepare(
            "SELECT video_id, start_time_ms, end_time_ms, content
             FROM subtitles
             WHERE content LIKE ?1
             ORDER BY video_id, start_time_ms ASC
             LIMIT ?2",
        )
        .map_err(|e| format!("Failed to prepare snippet search: {}", e))?;

    let rows = stmt
        .query_map(params![pattern, limit as i64], |row| {
            Ok(SubtitleSnippetMatch {
                video_id: row.get(0)?,
                start_time_ms: row.get(1)?,
                end_time_ms: row.get(2)?,
                content: row.get(3)?,
            })
        })
        .map_err(|e| format!("Failed to execute snippet search: {}", e))?;

    let mut list = Vec::new();
    for r in rows {
        if let Ok(item) = r {
            list.push(item);
        }
    }
    Ok(list)
}

// ==========================================
// Subtitle Formatting & Parsing Helpers
// ==========================================

pub fn format_ms_to_srt_time(ms: i64) -> String {
    let total_secs = (ms / 1000).max(0);
    let millis = (ms % 1000).max(0);
    let hours = total_secs / 3600;
    let mins = (total_secs % 3600) / 60;
    let secs = total_secs % 60;
    format!("{:02}:{:02}:{:02},{:03}", hours, mins, secs, millis)
}

pub fn format_ms_to_vtt_time(ms: i64) -> String {
    let total_secs = (ms / 1000).max(0);
    let millis = (ms % 1000).max(0);
    let hours = total_secs / 3600;
    let mins = (total_secs % 3600) / 60;
    let secs = total_secs % 60;
    format!("{:02}:{:02}:{:02}.{:03}", hours, mins, secs, millis)
}

pub fn export_to_srt(segments: &[SubtitleSegment]) -> String {
    let mut out = String::new();
    for (i, seg) in segments.iter().enumerate() {
        out.push_str(&format!("{}\n", i + 1));
        out.push_str(&format!(
            "{} --> {}\n",
            format_ms_to_srt_time(seg.start_time_ms),
            format_ms_to_srt_time(seg.end_time_ms)
        ));
        out.push_str(&format!("{}\n\n", seg.content.trim()));
    }
    out
}

pub fn export_to_vtt(segments: &[SubtitleSegment]) -> String {
    let mut out = String::from("WEBVTT\n\n");
    for (i, seg) in segments.iter().enumerate() {
        out.push_str(&format!("{}\n", i + 1));
        out.push_str(&format!(
            "{} --> {}\n",
            format_ms_to_vtt_time(seg.start_time_ms),
            format_ms_to_vtt_time(seg.end_time_ms)
        ));
        out.push_str(&format!("{}\n\n", seg.content.trim()));
    }
    out
}

/// Helper function to parse timestamp strings like "00:01:23,456" or "01:23.456" into milliseconds.
pub fn parse_timestamp_to_ms(time_str: &str) -> Option<i64> {
    let clean = time_str.trim().replace(',', ".");
    let parts: Vec<&str> = clean.split(':').collect();
    if parts.len() == 2 {
        // MM:SS.mmm
        let mins: f64 = parts[0].parse().ok()?;
        let secs: f64 = parts[1].parse().ok()?;
        Some(((mins * 60.0 + secs) * 1000.0).round() as i64)
    } else if parts.len() == 3 {
        // HH:MM:SS.mmm
        let hours: f64 = parts[0].parse().ok()?;
        let mins: f64 = parts[1].parse().ok()?;
        let secs: f64 = parts[2].parse().ok()?;
        Some(((hours * 3600.0 + mins * 60.0 + secs) * 1000.0).round() as i64)
    } else {
        None
    }
}

/// Parses an SRT or VTT text format into structured `SubtitleSegment` list.
pub fn parse_srt_or_vtt(text: &str, video_id: i64) -> Vec<SubtitleSegment> {
    let mut segments = Vec::new();
    let lines: Vec<&str> = text.lines().collect();
    let mut idx = 0;

    // Skip WEBVTT header if present
    if idx < lines.len() && lines[idx].trim().starts_with("WEBVTT") {
        idx += 1;
    }

    while idx < lines.len() {
        let line = lines[idx].trim();
        if line.is_empty() {
            idx += 1;
            continue;
        }

        // Check if current or next line has "-->"
        let timing_line = if line.contains("-->") {
            line
        } else if idx + 1 < lines.len() && lines[idx + 1].contains("-->") {
            idx += 1;
            lines[idx].trim()
        } else {
            idx += 1;
            continue;
        };

        let time_parts: Vec<&str> = timing_line.split("-->").collect();
        if time_parts.len() == 2 {
            let start_ms = parse_timestamp_to_ms(time_parts[0]);
            // End part might have extra cue settings e.g. "00:05.000 line:0 position:20%"
            let end_str = time_parts[1].trim().split_whitespace().next().unwrap_or("");
            let end_ms = parse_timestamp_to_ms(end_str);

            if let (Some(s_ms), Some(e_ms)) = (start_ms, end_ms) {
                idx += 1;
                let mut content_lines = Vec::new();
                while idx < lines.len() && !lines[idx].trim().is_empty() {
                    content_lines.push(lines[idx].trim());
                    idx += 1;
                }
                let content = content_lines.join(" ");
                if !content.is_empty() {
                    segments.push(SubtitleSegment {
                        id: None,
                        video_id,
                        language: None,
                        format: None,
                        start_time_ms: s_ms,
                        end_time_ms: e_ms,
                        content,
                    });
                }
            }
        }
        idx += 1;
    }

    segments
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn setup_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::migrations::apply_all(&conn).unwrap();
        conn
    }

    #[test]
    fn test_format_and_parse_timestamps() {
        let ms = 1 * 3600 * 1000 + 23 * 60 * 1000 + 45 * 1000 + 678;
        let srt_str = format_ms_to_srt_time(ms);
        assert_eq!(srt_str, "01:23:45,678");

        let parsed = parse_timestamp_to_ms(&srt_str).unwrap();
        assert_eq!(parsed, ms);

        let vtt_str = format_ms_to_vtt_time(ms);
        assert_eq!(vtt_str, "01:23:45.678");
        let parsed_vtt = parse_timestamp_to_ms(&vtt_str).unwrap();
        assert_eq!(parsed_vtt, ms);
    }

    #[test]
    fn test_parse_srt_or_vtt_string() {
        let srt = r#"
1
00:00:01,000 --> 00:00:04,500
Hello, world!
This is a test.

2
00:00:05,200 --> 00:00:08,000
Second subtitle line.
"#;
        let segments = parse_srt_or_vtt(srt, 42);
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].start_time_ms, 1000);
        assert_eq!(segments[0].end_time_ms, 4500);
        assert_eq!(segments[0].content, "Hello, world! This is a test.");
        assert_eq!(segments[1].start_time_ms, 5200);
        assert_eq!(segments[1].end_time_ms, 8000);
        assert_eq!(segments[1].content, "Second subtitle line.");
    }

    #[test]
    fn test_subtitles_crud_and_fts_sync() {
        let mut conn = setup_test_db();

        // 1. Insert a mock video
        conn.execute(
            "INSERT INTO videos (id, file_path, file_name, file_hash, file_size, created_at, updated_at)
             VALUES (1, '/movies/rust.mp4', 'rust.mp4', 'hash123', 1024, 100, 100)",
            [],
        )
        .unwrap();

        let segments = vec![
            SubtitleSegment {
                id: None,
                video_id: 1,
                language: Some("zh".to_string()),
                format: Some("vtt".to_string()),
                start_time_ms: 1200,
                end_time_ms: 3500,
                content: "欢迎来到 Rust 编程进阶教程 Ownership".to_string(),
            },
            SubtitleSegment {
                id: None,
                video_id: 1,
                language: Some("zh".to_string()),
                format: Some("vtt".to_string()),
                start_time_ms: 3600,
                end_time_ms: 6000,
                content: "今天我们将深入探讨所有权机制 Lifetime".to_string(),
            },
        ];

        let saved = save_video_subtitles(&mut conn, 1, Some("zh"), Some("vtt"), &segments).unwrap();
        assert_eq!(saved, 2);

        // Verify retrieval
        let retrieved = get_video_subtitles(&conn, 1).unwrap();
        assert_eq!(retrieved.len(), 2);
        assert!(retrieved[0].content.contains("Rust"));

        // Verify snippet search
        let snippets = search_subtitles_snippets(&conn, "所有权", 10).unwrap();
        assert_eq!(snippets.len(), 1);
        assert_eq!(snippets[0].video_id, 1);
        assert_eq!(snippets[0].start_time_ms, 3600);

        // Verify FTS5 index contains subtitles
        let fts_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM fts_index WHERE fts_index MATCH 'Ownership'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(fts_count, 1);

        // Delete subtitles
        delete_video_subtitles(&mut conn, 1).unwrap();
        let empty = get_video_subtitles(&conn, 1).unwrap();
        assert!(empty.is_empty());

        let fts_after: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM fts_index WHERE fts_index MATCH 'Ownership'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(fts_after, 0);
    }
}
