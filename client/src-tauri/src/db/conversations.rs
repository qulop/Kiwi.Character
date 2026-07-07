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
        messages::insert(conn, conversation_id, "assistant", &greeting)?;
    }
    Ok(())
}

pub fn character_id_of(conn: &Connection, conversation_id: &str) -> Result<String, String> {
    conn.query_row(
        "SELECT character_id FROM conversations WHERE id = ?1",
        params![conversation_id],
        |r| r.get(0),
    )
    .map_err(|e| e.to_string())
}
