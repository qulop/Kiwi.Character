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
const SCHEMA_VERSION: i64 = 2;

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

        let from_version: i64 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        conn.execute_batch(SCHEMA).map_err(|e| e.to_string())?;
        migrate(&conn, from_version)?;
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

/// Apply schema migrations for databases created before the current version.
/// The `CREATE TABLE IF NOT EXISTS` in schema.sql already covers fresh DBs, so
/// each step here only patches existing tables.
fn migrate(conn: &Connection, from_version: i64) -> Result<(), String> {
    // v2: messages gained a `hidden` flag (technical "continue" messages).
    if from_version < 2 {
        add_column_if_missing(conn, "messages", "hidden", "INTEGER NOT NULL DEFAULT 0")?;
    }
    Ok(())
}

/// `ALTER TABLE ... ADD COLUMN` only when the column isn't already present.
/// (`table`/`column`/`decl` are internal constants — no injection risk.)
fn add_column_if_missing(
    conn: &Connection,
    table: &str,
    column: &str,
    decl: &str,
) -> Result<(), String> {
    let mut stmt = conn
        .prepare(&format!("PRAGMA table_info({table})"))
        .map_err(|e| e.to_string())?;
    let exists = stmt
        .query_map([], |r| r.get::<_, String>(1)) // column 1 = name
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .any(|name| name == column);
    if !exists {
        conn.execute(
            &format!("ALTER TABLE {table} ADD COLUMN {column} {decl}"),
            [],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}
