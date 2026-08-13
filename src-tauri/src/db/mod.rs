//! SQLite persistence layer.
//!
//! One `rusqlite::Connection` (guarded by a `Mutex` in `AppState`) plus small
//! per-entity repository modules. The authoritative schema lives in
//! `agent-docs/database-scheme.md`; `schema.sql` mirrors it.

use std::path::{Path, PathBuf};

use rusqlite::Connection;

pub mod characters;
pub mod conversations;
pub mod groups;
pub mod messages;
pub mod personas;
pub mod settings;

const SCHEMA: &str = include_str!("schema.sql");
const SCHEMA_VERSION: i64 = 3;

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
        return Ok(db);
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
        return Ok(());
    }
}

/// Decode a `data:` URL and write it under `avatars/`. Returns the relative path.
/// Shared by any repository that stores an avatar image (characters, personas).
///
/// The filename embeds a fresh UUID so every save produces a genuinely new
/// file/URL — re-uploading an avatar for the same character or persona used
/// to reuse the exact `<id>.<ext>` path, so the webview kept showing its
/// cached copy of the old image until a full reload. Callers that replace an
/// existing avatar (see `characters::update`/`personas::update`) already
/// delete the previous file, so this doesn't leak orphaned files. (A
/// timestamp alone isn't quite enough — two saves within the same
/// millisecond would still collide.)
pub(crate) fn save_avatar(dir: &Path, id: &str, data_url: &str) -> Result<String, String> {
    use base64::{engine::general_purpose::STANDARD, Engine};

    let (meta, b64) = data_url
        .split_once(',')
        .ok_or("avatar is not a data URL")?;
    let ext = if meta.contains("jpeg") || meta.contains("jpg") {
        "jpg"
    } else if meta.contains("webp") {
        "webp"
    } else {
        "png"
    };
    let bytes = STANDARD.decode(b64.trim()).map_err(|e| e.to_string())?;
    let file = format!("{id}-{}.{ext}", crate::state::new_id());
    std::fs::write(dir.join(&file), bytes).map_err(|e| e.to_string())?;
    return Ok(format!("avatars/{file}"));
}

/// Best-effort delete of a stored avatar file, given its `avatars/<file>` relative path.
pub(crate) fn remove_avatar_file(dir: &Path, rel: &str) {
    let file = rel.strip_prefix("avatars/").unwrap_or(rel);
    let _ = std::fs::remove_file(dir.join(file));
}

/// Apply schema migrations for databases created before the current version.
/// The `CREATE TABLE IF NOT EXISTS` in schema.sql already covers fresh DBs, so
/// each step here only patches existing tables.
fn migrate(conn: &Connection, from_version: i64) -> Result<(), String> {
    // v2: messages gained a `hidden` flag (technical "continue" messages).
    if from_version < 2 {
        add_column_if_missing(conn, "messages", "hidden", "INTEGER NOT NULL DEFAULT 0")?;
    }
    // v3: conversations gained an active_persona_id (which persona is selected
    // for that chat), remembered across launches.
    if from_version < 3 {
        add_column_if_missing(conn, "conversations", "active_persona_id", "TEXT")?;
    }
    return Ok(());
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
    return Ok(());
}

#[cfg(test)]
mod tests {
    use super::*;

    const TINY_PNG_DATA_URL: &str =
        "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=";

    #[test]
    fn save_avatar_never_collides_across_calls() {
        let dir = std::env::temp_dir().join(format!("kiwi-avatar-test-{}", crate::state::new_id()));
        std::fs::create_dir_all(&dir).unwrap();

        let path1 = save_avatar(&dir, "char1", TINY_PNG_DATA_URL).unwrap();
        // A second save for the SAME id must produce a different file — this is
        // exactly the bug: identical filenames meant the webview kept showing
        // its cached copy of the old image after an avatar edit.
        let path2 = save_avatar(&dir, "char1", TINY_PNG_DATA_URL).unwrap();
        assert_ne!(path1, path2);
        assert!(dir.join(path1.strip_prefix("avatars/").unwrap()).exists());
        assert!(dir.join(path2.strip_prefix("avatars/").unwrap()).exists());

        std::fs::remove_dir_all(&dir).ok();
    }
}
