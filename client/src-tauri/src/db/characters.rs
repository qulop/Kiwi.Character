//! `characters` table repository.

use std::path::Path;

use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::models::{Character, NewCharacterInput};
use crate::state::{new_id, now_ms};

/// Shared projection: every character column plus the newest message time
/// across its conversations (`last_message_at`, NULL if none). Used by both
/// `list` and `get` so the row mapping always finds the same columns.
const SELECT_CHARACTER: &str = "
    SELECT c.id, c.name, c.info, c.avatar_path, c.appearance, c.description,
           c.initial_message, c.is_favorite, c.created_at,
           v.last_at AS last_message_at
    FROM characters c
    LEFT JOIN (
        SELECT character_id, MAX(last_message_at) AS last_at
        FROM conversations GROUP BY character_id
    ) v ON v.character_id = c.id";

fn row_to_character(r: &Row) -> rusqlite::Result<Character> {
    Ok(Character {
        id: r.get("id")?,
        name: r.get("name")?,
        info: r.get("info")?,
        avatar: r.get::<_, Option<String>>("avatar_path")?, // relative path or None
        appearance: r.get("appearance")?,
        description: r.get("description")?,
        initial_message: r.get("initial_message")?,
        is_favorite: r.get::<_, i64>("is_favorite")? != 0,
        created_at: r.get("created_at")?,
        last_message_at: r.get::<_, Option<i64>>("last_message_at")?,
    })
}

pub fn list(conn: &Connection) -> Result<Vec<Character>, String> {
    let sql = format!("{SELECT_CHARACTER} ORDER BY c.created_at DESC");
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], row_to_character).map_err(|e| e.to_string())?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| e.to_string())
}

pub fn get(conn: &Connection, id: &str) -> Result<Option<Character>, String> {
    let sql = format!("{SELECT_CHARACTER} WHERE c.id = ?1");
    conn.query_row(&sql, params![id], row_to_character)
        .optional()
        .map_err(|e| e.to_string())
}

/// Case-insensitive existence check. Pass `""` for `exclude_id` when creating.
pub fn name_exists(conn: &Connection, name: &str, exclude_id: &str) -> Result<bool, String> {
    conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM characters WHERE name = ?1 COLLATE NOCASE AND id <> ?2)",
        params![name, exclude_id],
        |r| r.get::<_, i64>(0),
    )
    .map(|n| n == 1)
    .map_err(|e| e.to_string())
}

pub fn insert(
    conn: &Connection,
    avatars_dir: &Path,
    input: NewCharacterInput,
) -> Result<Character, String> {
    // Names must be unique case-insensitively (Step 6). Trim first.
    let name = input.name.trim().to_string();
    if name.is_empty() {
        return Err("Character name is required".into());
    }
    if name_exists(conn, &name, "")? {
        return Err(format!("A character named '{name}' already exists"));
    }

    let id = new_id();
    let ts = now_ms();

    // Persist the avatar file if a data URL was provided.
    let avatar_path = match input.avatar.as_deref() {
        Some(data) if !data.is_empty() => Some(save_avatar(avatars_dir, &id, data)?),
        _ => None,
    };

    conn.execute(
        "INSERT INTO characters
           (id, name, info, avatar_path, appearance, description, initial_message,
            is_favorite, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, ?8, ?8)",
        params![
            id,
            name,
            input.info,
            avatar_path,
            input.appearance,
            input.description,
            input.initial_message,
            ts
        ],
    )
    // Map the UNIQUE-index violation (covers a check→insert race) to the same
    // friendly message.
    .map_err(|err| {
        let msg = err.to_string();
        if msg.contains("UNIQUE") {
            format!("A character named '{name}' already exists")
        } else {
            msg
        }
    })?;

    Ok(Character {
        id,
        name,
        info: input.info,
        avatar: avatar_path,
        appearance: input.appearance,
        description: input.description,
        initial_message: input.initial_message,
        is_favorite: false,
        created_at: ts,
        last_message_at: None,
    })
}

/// Delete a character. Removes its avatar file (best-effort); the row delete
/// cascades to conversations and messages (with `PRAGMA foreign_keys=ON`).
pub fn delete(conn: &Connection, avatars_dir: &Path, id: &str) -> Result<(), String> {
    if let Ok(Some(c)) = get(conn, id) {
        if let Some(rel) = c.avatar {
            let file = rel.strip_prefix("avatars/").unwrap_or(&rel);
            let _ = std::fs::remove_file(avatars_dir.join(file));
        }
    }
    conn.execute("DELETE FROM characters WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn set_favorite(conn: &Connection, id: &str, fav: bool) -> Result<(), String> {
    conn.execute(
        "UPDATE characters SET is_favorite = ?2, updated_at = ?3 WHERE id = ?1",
        params![id, fav as i64, now_ms()],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Decode a `data:` URL and write it under `avatars/`. Returns the relative path.
fn save_avatar(dir: &Path, id: &str, data_url: &str) -> Result<String, String> {
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
    let file = format!("{id}.{ext}");
    std::fs::write(dir.join(&file), bytes).map_err(|e| e.to_string())?;
    Ok(format!("avatars/{file}"))
}

/// First-run sample characters (stable ids so `conv-<id>` lines up).
pub fn seed_samples(conn: &Connection) -> Result<(), String> {
    let ts = now_ms();
    let samples: [(&str, &str, &str, &str, &str, &str); 3] = [
        (
            "aria",
            "Aria",
            "Curious AI companion who loves to learn",
            "A warm, holographic presence with shifting violet hues.",
            "Aria is endlessly curious, upbeat, and a little playful. She asks thoughtful \
             follow-up questions and enjoys a good tangent.",
            "Hi there! I'm Aria. What's on your mind today?",
        ),
        (
            "sherlock",
            "Sherlock Holmes",
            "Brilliant, observant consulting detective",
            "Tall and lean, sharp features, often in a long coat.",
            "The world's only consulting detective. Deductive, blunt, impatient with sloppy \
             thinking, but loyal to those he respects.",
            "Ah, a visitor. Sit. You've clearly come about a problem — out with it.",
        ),
        (
            "luna",
            "Luna",
            "Dreamy poet with a love for the night sky",
            "Soft silver hair, star-flecked eyes, always a little distant.",
            "Luna speaks in gentle, lyrical language and finds wonder in small things. Calm, \
             kind, and quietly wise.",
            "Oh, hello… I was just watching the stars. Care to join me?",
        ),
    ];

    for (id, name, info, appearance, description, greeting) in samples {
        conn.execute(
            "INSERT INTO characters
               (id, name, info, avatar_path, appearance, description, initial_message,
                is_favorite, created_at, updated_at)
             VALUES (?1, ?2, ?3, NULL, ?4, ?5, ?6, 0, ?7, ?7)",
            params![id, name, info, appearance, description, greeting, ts],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}
