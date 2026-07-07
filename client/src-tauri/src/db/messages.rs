//! `messages` table repository.

use rusqlite::{params, Connection};

use crate::models::ChatMessage;
use crate::state::{new_id, now_ms};

pub fn list(conn: &Connection, conversation_id: &str) -> Result<Vec<ChatMessage>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, role, content, created_at FROM messages
             WHERE conversation_id = ?1 ORDER BY created_at ASC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![conversation_id], |r| {
            Ok(ChatMessage {
                id: r.get(0)?,
                role: r.get(1)?,
                content: r.get(2)?,
                created_at: Some(r.get(3)?),
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| e.to_string())
}

/// Insert a message and bump the parent conversation's `last_message_at`.
pub fn insert(
    conn: &Connection,
    conversation_id: &str,
    role: &str,
    content: &str,
) -> Result<ChatMessage, String> {
    let m = ChatMessage {
        id: new_id(),
        role: role.into(),
        content: content.into(),
        created_at: Some(now_ms()),
    };
    conn.execute(
        "INSERT INTO messages (id, conversation_id, role, content, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![m.id, conversation_id, m.role, m.content, m.created_at],
    )
    .map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE conversations SET last_message_at = ?2 WHERE id = ?1",
        params![conversation_id, m.created_at],
    )
    .map_err(|e| e.to_string())?;
    Ok(m)
}
