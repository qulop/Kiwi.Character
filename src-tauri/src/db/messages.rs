//! `messages` table repository.

use rusqlite::{params, Connection, OptionalExtension};

use crate::models::ChatMessage;
use crate::state::{new_id, now_ms};

fn map_rows(
    stmt: &mut rusqlite::Statement,
    conversation_id: &str,
) -> Result<Vec<ChatMessage>, String> {
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
    return rows
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| e.to_string());
}

/// Visible messages only (hidden technical messages excluded) — for the UI.
pub fn list(conn: &Connection, conversation_id: &str) -> Result<Vec<ChatMessage>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, role, content, created_at FROM messages
             WHERE conversation_id = ?1 AND hidden = 0 ORDER BY created_at ASC, rowid ASC",
        )
        .map_err(|e| e.to_string())?;
    return map_rows(&mut stmt, conversation_id);
}

/// All messages including hidden ones — for building the model prompt.
pub fn list_all(conn: &Connection, conversation_id: &str) -> Result<Vec<ChatMessage>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, role, content, created_at FROM messages
             WHERE conversation_id = ?1 ORDER BY created_at ASC, rowid ASC",
        )
        .map_err(|e| e.to_string())?;
    return map_rows(&mut stmt, conversation_id);
}

/// The role of the newest message (by insertion order), if any.
pub fn last_role(conn: &Connection, conversation_id: &str) -> Result<Option<String>, String> {
    return conn.query_row(
        "SELECT role FROM messages WHERE conversation_id = ?1 ORDER BY rowid DESC LIMIT 1",
        params![conversation_id],
        |r| r.get::<_, String>(0),
    )
    .optional()
    .map_err(|e| e.to_string());
}

/// Insert a message and bump the parent conversation's `last_message_at`.
/// `hidden` marks a technical message that the UI must not display.
pub fn insert(
    conn: &Connection,
    conversation_id: &str,
    role: &str,
    content: &str,
    hidden: bool,
) -> Result<ChatMessage, String> {
    let m = ChatMessage {
        id: new_id(),
        role: role.into(),
        content: content.into(),
        created_at: Some(now_ms()),
    };
    conn.execute(
        "INSERT INTO messages (id, conversation_id, role, content, created_at, hidden)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![m.id, conversation_id, m.role, m.content, m.created_at, hidden as i64],
    )
    .map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE conversations SET last_message_at = ?2 WHERE id = ?1",
        params![conversation_id, m.created_at],
    )
    .map_err(|e| e.to_string())?;
    return Ok(m);
}

/// Update a message's text in place.
pub fn update_content(conn: &Connection, message_id: &str, content: &str) -> Result<(), String> {
    conn.execute(
        "UPDATE messages SET content = ?2 WHERE id = ?1",
        params![message_id, content],
    )
    .map_err(|e| e.to_string())?;
    return Ok(());
}

/// Delete a single message, then refresh the conversation's last_message_at.
pub fn delete(conn: &Connection, conversation_id: &str, message_id: &str) -> Result<(), String> {
    conn.execute(
        "DELETE FROM messages WHERE id = ?1 AND conversation_id = ?2",
        params![message_id, conversation_id],
    )
    .map_err(|e| e.to_string())?;
    return refresh_last_message_at(conn, conversation_id);
}

/// Delete every message positioned AFTER `message_id` in the same conversation.
/// Uses rowid (insertion order) so equal-millisecond timestamps can't misorder.
pub fn rewind(conn: &Connection, conversation_id: &str, message_id: &str) -> Result<(), String> {
    conn.execute(
        "DELETE FROM messages
         WHERE conversation_id = ?1
           AND rowid > (SELECT rowid FROM messages WHERE id = ?2)",
        params![conversation_id, message_id],
    )
    .map_err(|e| e.to_string())?;
    return refresh_last_message_at(conn, conversation_id);
}

/// Set last_message_at to the newest remaining message time (NULL if none).
fn refresh_last_message_at(conn: &Connection, conversation_id: &str) -> Result<(), String> {
    conn.execute(
        "UPDATE conversations
         SET last_message_at = (SELECT MAX(created_at) FROM messages WHERE conversation_id = ?1)
         WHERE id = ?1",
        params![conversation_id],
    )
    .map_err(|e| e.to_string())?;
    return Ok(());
}
