use rusqlite::Connection;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct Migration {
    pub version: i32,
    pub name: &'static str,
    pub sql: &'static str,
}

pub const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        name: "v1_initial_schema",
        sql: r#"
        -- 核心视频表
        CREATE TABLE IF NOT EXISTS videos (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            file_path TEXT NOT NULL UNIQUE,
            file_name TEXT NOT NULL,
            file_hash TEXT NOT NULL,
            file_size INTEGER NOT NULL,
            duration REAL,
            width INTEGER,
            height INTEGER,
            codec TEXT,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            status TEXT NOT NULL DEFAULT 'ready',
            thumbnail_path TEXT
        );

        CREATE INDEX IF NOT EXISTS idx_videos_hash ON videos(file_hash);
        CREATE INDEX IF NOT EXISTS idx_videos_created ON videos(created_at);

        -- 视频来源追溯表
        CREATE TABLE IF NOT EXISTS video_sources (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            video_id INTEGER NOT NULL REFERENCES videos(id) ON DELETE CASCADE,
            original_url TEXT,
            platform TEXT,
            author TEXT,
            download_task_id TEXT,
            downloaded_at INTEGER
        );

        -- 字幕解析与区间索引表
        CREATE TABLE IF NOT EXISTS subtitles (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            video_id INTEGER NOT NULL REFERENCES videos(id) ON DELETE CASCADE,
            language TEXT,
            format TEXT,
            start_time_ms INTEGER NOT NULL,
            end_time_ms INTEGER NOT NULL,
            content TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_subtitles_video ON subtitles(video_id);

        -- 本地 AI 分析与增强表
        CREATE TABLE IF NOT EXISTS ai_analyses (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            video_id INTEGER NOT NULL UNIQUE REFERENCES videos(id) ON DELETE CASCADE,
            summary TEXT,
            keywords TEXT,
            tags TEXT,
            model_name TEXT,
            created_at INTEGER NOT NULL
        );

        -- 下载任务状态队列持久化表
        CREATE TABLE IF NOT EXISTS download_tasks (
            id TEXT PRIMARY KEY,
            url TEXT NOT NULL,
            status TEXT NOT NULL,
            progress REAL NOT NULL DEFAULT 0.0,
            total_bytes INTEGER DEFAULT 0,
            speed REAL DEFAULT 0.0,
            error_log TEXT,
            priority INTEGER DEFAULT 0,
            retry_count INTEGER DEFAULT 0,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        );

        -- FTS5 全文检索虚表
        CREATE VIRTUAL TABLE IF NOT EXISTS fts_index USING fts5(
            video_id UNINDEXED,
            title,
            subtitles,
            summary,
            tags
        );
        "#,
    },
    Migration {
        version: 2,
        name: "v2_download_tasks_fields",
        sql: r#"
        ALTER TABLE download_tasks ADD COLUMN title TEXT;
        ALTER TABLE download_tasks ADD COLUMN author TEXT;
        ALTER TABLE download_tasks ADD COLUMN platform TEXT;
        ALTER TABLE download_tasks ADD COLUMN format_id TEXT;
        ALTER TABLE download_tasks ADD COLUMN file_path TEXT;
        ALTER TABLE download_tasks ADD COLUMN downloaded_bytes INTEGER DEFAULT 0;
        ALTER TABLE download_tasks ADD COLUMN speed_str TEXT;
        ALTER TABLE download_tasks ADD COLUMN eta TEXT;
        "#,
    },
    Migration {
        version: 3,
        name: "v3_extracted_audios",
        sql: r#"
        CREATE TABLE IF NOT EXISTS extracted_audios (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            video_id INTEGER REFERENCES videos(id) ON DELETE SET NULL,
            source_video_title TEXT,
            source_video_path TEXT,
            file_path TEXT NOT NULL UNIQUE,
            file_name TEXT NOT NULL,
            file_size INTEGER NOT NULL,
            duration REAL,
            format TEXT NOT NULL,
            quality_bitrate TEXT,
            created_at INTEGER NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_extracted_audios_created ON extracted_audios(created_at);
        CREATE INDEX IF NOT EXISTS idx_extracted_audios_format ON extracted_audios(format);
        "#,
    },
];

pub fn apply_all(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            applied_at INTEGER NOT NULL
        );",
    )
    .map_err(|e| format!("Failed to create schema_migrations table: {}", e))?;

    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;

    for migration in MIGRATIONS {
        let already_applied: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version = ?1)",
                [migration.version],
                |row| row.get(0),
            )
            .unwrap_or(false);

        if !already_applied {
            conn.execute_batch(migration.sql).map_err(|e| {
                format!(
                    "Failed executing migration {} ({}): {}",
                    migration.version, migration.name, e
                )
            })?;

            conn.execute(
                "INSERT INTO schema_migrations (version, name, applied_at) VALUES (?1, ?2, ?3)",
                rusqlite::params![migration.version, migration.name, now_ms],
            )
            .map_err(|e| format!("Failed recording migration {}: {}", migration.version, e))?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migrations_and_fts5_in_memory() {
        let conn = Connection::open_in_memory().expect("Failed to open memory db");
        apply_all(&conn).expect("Failed to apply migrations");

        // Verify version 1 and 2 recorded
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM schema_migrations WHERE version = 1", [], |r| r.get(0))
            .expect("Query migrations");
        assert_eq!(count, 1);

        let count_v2: i64 = conn
            .query_row("SELECT COUNT(*) FROM schema_migrations WHERE version = 2", [], |r| r.get(0))
            .expect("Query migrations");
        assert_eq!(count_v2, 1);

        // Verify tables exist
        let tables_exist: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='videos')",
                [],
                |r| r.get(0),
            )
            .expect("Query tables");
        assert!(tables_exist);

        // Verify FTS5 virtual table works
        conn.execute(
            "INSERT INTO fts_index (video_id, title, subtitles, summary, tags) VALUES ('1', 'Tutorial Video', 'Hello world intro', 'Summary test', 'rust,ai')",
            [],
        )
        .expect("Insert into fts5");

        let fts_match: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM fts_index WHERE fts_index MATCH 'Tutorial*'",
                [],
                |r| r.get(0),
            )
            .expect("Query fts5");
        assert_eq!(fts_match, 1);
    }
}
