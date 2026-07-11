//! `personas` table repository.

use std::path::Path;

use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::models::{NewPersonaInput, Persona};
use crate::state::{new_id, now_ms};

fn row_to_persona(r: &Row) -> rusqlite::Result<Persona> {
    Ok(Persona {
        id: r.get("id")?,
        name: r.get("name")?,
        description: r.get("description")?,
        avatar: r.get::<_, Option<String>>("avatar_path")?,
        is_default: r.get::<_, i64>("is_default")? != 0,
        created_at: r.get("created_at")?,
    })
}

pub fn list(conn: &Connection) -> Result<Vec<Persona>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, name, description, avatar_path, is_default, created_at
             FROM personas ORDER BY created_at DESC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], row_to_persona)
        .map_err(|e| e.to_string())?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| e.to_string())
}

pub fn insert(
    conn: &Connection,
    avatars_dir: &Path,
    input: NewPersonaInput,
) -> Result<Persona, String> {
    let name = input.name.trim().to_string();
    if name.is_empty() {
        return Err("Persona name is required".into());
    }

    let id = new_id();
    let ts = now_ms();

    let avatar_path = match input.avatar.as_deref() {
        Some(data) if !data.is_empty() => Some(super::save_avatar(avatars_dir, &id, data)?),
        _ => None,
    };

    conn.execute(
        "INSERT INTO personas (id, name, description, avatar_path, is_default, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, 0, ?5, ?5)",
        params![id, name, input.description, avatar_path, ts],
    )
    .map_err(|e| e.to_string())?;

    Ok(Persona {
        id,
        name,
        description: input.description,
        avatar: avatar_path,
        is_default: false,
        created_at: ts,
    })
}

fn avatar_of(conn: &Connection, id: &str) -> Result<Option<String>, String> {
    conn.query_row(
        "SELECT avatar_path FROM personas WHERE id = ?1",
        params![id],
        |r| r.get::<_, Option<String>>(0),
    )
    .optional()
    .map_err(|e| e.to_string())
    .map(|opt| opt.flatten())
}

/// Delete a persona. Removes its avatar file (best-effort).
pub fn delete(conn: &Connection, avatars_dir: &Path, id: &str) -> Result<(), String> {
    if let Some(rel) = avatar_of(conn, id)? {
        super::remove_avatar_file(avatars_dir, &rel);
    }
    conn.execute("DELETE FROM personas WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}
