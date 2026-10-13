pub mod audios;
pub mod classification;
pub mod migrations;
pub mod subtitles;
pub mod tasks;
pub mod videos;

use rusqlite::{Connection, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct Database {
    db_path: PathBuf,
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    /// Initializes the database in the target data directory and executes all pending migrations.
    pub fn init(app_data_dir: &Path) -> Result<Self, String> {
        let data_dir = app_data_dir.join("data");
        fs::create_dir_all(&data_dir)
            .map_err(|e| format!("Failed to create database directory: {}", e))?;

        let db_path = data_dir.join("vidnest.db");
        let conn = Connection::open(&db_path)
            .map_err(|e| format!("Failed to open SQLite database: {}", e))?;

        // Enable foreign keys and WAL mode for high concurrency
        conn.execute_batch(
            "PRAGMA foreign_keys = ON;
             PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;",
        )
        .map_err(|e| format!("Failed to configure SQLite pragmas: {}", e))?;

        let mut db = Self {
            db_path,
            conn: Arc::new(Mutex::new(conn)),
        };

        db.run_migrations()?;
        Ok(db)
    }

    /// Creates an in-memory database for unit tests.
    pub fn init_in_memory() -> Result<Self, String> {
        let conn = Connection::open_in_memory()
            .map_err(|e| format!("Failed to open in-memory database: {}", e))?;

        conn.execute_batch(
            "PRAGMA foreign_keys = ON;
             PRAGMA synchronous = NORMAL;",
        )
        .map_err(|e| format!("Failed to configure pragmas: {}", e))?;

        let mut db = Self {
            db_path: PathBuf::from(":memory:"),
            conn: Arc::new(Mutex::new(conn)),
        };

        db.run_migrations()?;
        Ok(db)
    }

    /// Runs all schema migrations sequentially.
    pub fn run_migrations(&mut self) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        migrations::apply_all(&conn)
    }

    /// Provides access to the underlying SQLite connection for queries.
    pub fn get_connection(&self) -> Arc<Mutex<Connection>> {
        self.conn.clone()
    }

    pub fn get_path(&self) -> &Path {
        &self.db_path
    }
}
