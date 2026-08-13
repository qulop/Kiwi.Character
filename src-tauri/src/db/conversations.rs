//! `conversations` table repository.

use rusqlite::{params, Connection, OptionalExtension};

use super::messages;
use crate::state::now_ms;

/// Create the conversation lazily if missing, seeding it with the character's
/// greeting as the first assistant message. Frontend ids look like
/// `conv-<characterId>`.
pub fn ensure(conn: &Connection, conversation_id: &str) -> Result<(), String> {
    let exists: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM conversations WHERE id = ?1)",
            params![conversation_id],
            |r| r.get::<_, i64>(0),
        )
        .map(|n| n == 1)
        .map_err(|e| e.to_string())?;
    if exists {
        return Ok(());
    }

    let character_id = conversation_id
        .strip_prefix("conv-")
        .unwrap_or(conversation_id)
        .to_string();

    // Look up the greeting; None here means the character doesn't exist.
    let greeting: String = conn
        .query_row(
            "SELECT initial_message FROM characters WHERE id = ?1",
            params![character_id],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("No character '{character_id}' for '{conversation_id}'"))?;

    conn.execute(
        "INSERT INTO conversations (id, character_id, created_at, last_message_at)
         VALUES (?1, ?2, ?3, NULL)",
        params![conversation_id, character_id, now_ms()],
    )
    .map_err(|e| e.to_string())?;

    if !greeting.is_empty() {
        messages::insert(conn, conversation_id, "assistant", &greeting, false)?;
    }
    return Ok(());
}

pub fn character_id_of(conn: &Connection, conversation_id: &str) -> Result<String, String> {
    return conn.query_row(
        "SELECT character_id FROM conversations WHERE id = ?1",
        params![conversation_id],
        |r| r.get(0),
    )
    .map_err(|e| e.to_string());
}

/// The persona currently selected for this chat, if any.
pub fn active_persona_id(conn: &Connection, conversation_id: &str) -> Result<Option<String>, String> {
    return conn.query_row(
        "SELECT active_persona_id FROM conversations WHERE id = ?1",
        params![conversation_id],
        |r| r.get::<_, Option<String>>(0),
    )
    .optional()
    .map_err(|e| e.to_string())
    .map(|opt| opt.flatten());
}

/// Set (or clear, with `None`) the persona selected for this chat.
pub fn set_active_persona(
    conn: &Connection,
    conversation_id: &str,
    persona_id: Option<&str>,
) -> Result<(), String> {
    conn.execute(
        "UPDATE conversations SET active_persona_id = ?2 WHERE id = ?1",
        params![conversation_id, persona_id],
    )
    .map_err(|e| e.to_string())?;
    return Ok(());
}

/// Delete a conversation (and its messages, via cascade). The character stays;
/// opening it again re-creates a fresh conversation with the greeting.
pub fn delete(conn: &Connection, conversation_id: &str) -> Result<(), String> {
    conn.execute(
        "DELETE FROM conversations WHERE id = ?1",
        params![conversation_id],
    )
    .map_err(|e| e.to_string())?;
    return Ok(());
}
