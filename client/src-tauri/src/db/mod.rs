//! SQLite persistence layer.
//!
//! One `rusqlite::Connection` (guarded by a `Mutex` in `AppState`) plus small
//! per-entity repository modules. The authoritative schema lives in
//! `agent-docs/database-scheme.md`; `schema.sql` mirrors it.

use std::path::{Path, PathBuf};

use rusqlite::Connection;

pub mod characters;
pub mod conversations;
pub mod messages;
pub mod settings;

const SCHEMA: &str = include_str!("schema.sql");
const SCHEMA_VERSION: i64 = 1;

/// Owns the database connection and knows where avatar files live.
pub struct Db {
    pub conn: Connection,
    /// `<app_data_dir>/avatars`
    pub avatars_dir: PathBuf,
}

impl Db {
    /// Open (creating if needed) the database at `db_path`, run migrations, and
    /// ensure the singleton settings row + first-run seed data exist.
    pub fn open(db_path: &Path, avatars_dir: PathBuf) -> Result<Self, String> {
        std::fs::create_dir_all(&avatars_dir).map_err(|e| e.to_string())?;
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }

        let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
        // Pragmas that return a row (journal_mode) are safe inside execute_batch,
        // which discards result sets.
        conn.execute_batch("PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL;")
            .map_err(|e| e.to_string())?;
        conn.execute_batch(SCHEMA).map_err(|e| e.to_string())?;
        conn.pragma_update(None, "user_version", SCHEMA_VERSION)
            .map_err(|e| e.to_string())?;

        let db = Db { conn, avatars_dir };
        settings::ensure_row(&db.conn)?;
        db.seed_if_empty()?;
        Ok(db)
    }

    /// Populate a friendly set of sample characters on first run (empty DB).
    fn seed_if_empty(&self) -> Result<(), String> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM characters", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        if count == 0 {
            characters::seed_samples(&self.conn)?;
        }
        Ok(())
    }
}
