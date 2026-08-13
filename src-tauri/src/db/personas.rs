//! `personas` table repository.

use std::path::Path;

use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::models::{NewPersonaInput, Persona};
use crate::state::{new_id, now_ms};

fn row_to_persona(r: &Row) -> rusqlite::Result<Persona> {
    return Ok(Persona {
        id: r.get("id")?,
        name: r.get("name")?,
        description: r.get("description")?,
        avatar: r.get::<_, Option<String>>("avatar_path")?,
        is_default: r.get::<_, i64>("is_default")? != 0,
        created_at: r.get("created_at")?,
    });
}

const SELECT_PERSONA: &str =
    "SELECT id, name, description, avatar_path, is_default, created_at FROM personas";

pub fn list(conn: &Connection) -> Result<Vec<Persona>, String> {
    let sql = format!("{SELECT_PERSONA} ORDER BY created_at DESC");
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], row_to_persona)
        .map_err(|e| e.to_string())?;
    return rows
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| e.to_string());
}

pub fn get(conn: &Connection, id: &str) -> Result<Option<Persona>, String> {
    let sql = format!("{SELECT_PERSONA} WHERE id = ?1");
    return conn
        .query_row(&sql, params![id], row_to_persona)
        .optional()
        .map_err(|e| e.to_string());
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

    return Ok(Persona {
        id,
        name,
        description: input.description,
        avatar: avatar_path,
        is_default: false,
        created_at: ts,
    });
}

/// Update an existing persona. `input.avatar`:
///   - a "data:" URL -> decode, save a new file, replace avatar_path (delete old)
///   - anything else  -> leave the stored avatar untouched
pub fn update(
    conn: &Connection,
    avatars_dir: &Path,
    id: &str,
    input: NewPersonaInput,
) -> Result<Persona, String> {
    let name = input.name.trim().to_string();
    if name.is_empty() {
        return Err("Persona name is required".into());
    }

    let new_avatar = match input.avatar.as_deref() {
        Some(a) if a.starts_with("data:") => Some(super::save_avatar(avatars_dir, id, a)?),
        _ => None,
    };

    if let Some(path) = &new_avatar {
        if let Some(prev) = get(conn, id)?.and_then(|p| p.avatar) {
            if prev.as_str() != path.as_str() {
                super::remove_avatar_file(avatars_dir, &prev);
            }
        }
        conn.execute(
            "UPDATE personas SET name=?2, description=?3, avatar_path=?4, updated_at=?5 WHERE id=?1",
            params![id, name, input.description, path, now_ms()],
        )
        .map_err(|e| e.to_string())?;
    } else {
        conn.execute(
            "UPDATE personas SET name=?2, description=?3, updated_at=?4 WHERE id=?1",
            params![id, name, input.description, now_ms()],
        )
        .map_err(|e| e.to_string())?;
    }

    return get(conn, id)?.ok_or_else(|| format!("Persona '{id}' not found"));
}

fn avatar_of(conn: &Connection, id: &str) -> Result<Option<String>, String> {
    return conn.query_row(
        "SELECT avatar_path FROM personas WHERE id = ?1",
        params![id],
        |r| r.get::<_, Option<String>>(0),
    )
    .optional()
    .map_err(|e| e.to_string())
    .map(|opt| opt.flatten());
}

/// Delete a persona. Removes its avatar file (best-effort) and clears any
/// conversation that had it selected as the active persona.
pub fn delete(conn: &Connection, avatars_dir: &Path, id: &str) -> Result<(), String> {
    if let Some(rel) = avatar_of(conn, id)? {
        super::remove_avatar_file(avatars_dir, &rel);
    }
    conn.execute(
        "UPDATE conversations SET active_persona_id = NULL WHERE active_persona_id = ?1",
        params![id],
    )
    .map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM personas WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    return Ok(());
}
