//! SQLite repository for long-term memory and its singleton settings.

use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::models::{Memory, MemoryCandidate, MemoryEntryStatus, MemorySettings, NewMemoryInput};
use crate::state::{new_id, now_ms};

const MEMORY_COLUMNS: &str = "id, conversation_id, character_id, persona_id, kind, content,
    embedding_dimensions, embedding_model, importance, pinned, status,
    created_at, updated_at, last_recalled_at";
const SELECT_MEMORY: &str = "SELECT id, conversation_id, character_id, persona_id, kind, content,
    embedding_dimensions, embedding_model, importance, pinned, status,
    created_at, updated_at, last_recalled_at FROM memories";

fn row_to_memory(row: &Row) -> rusqlite::Result<Memory> {
    Ok(Memory {
        id: row.get("id")?,
        conversation_id: row.get("conversation_id")?,
        character_id: row.get("character_id")?,
        persona_id: row.get("persona_id")?,
        kind: row.get("kind")?,
        content: row.get("content")?,
        embedding_dimensions: row.get("embedding_dimensions")?,
        embedding_model: row.get("embedding_model")?,
        importance: row.get("importance")?,
        pinned: row.get::<_, i64>("pinned")? != 0,
        status: row.get("status")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
        last_recalled_at: row.get("last_recalled_at")?,
    })
}

pub fn ensure_settings_row(conn: &Connection) -> Result<(), String> {
    let settings = MemorySettings::default();
    conn.execute(
        "INSERT OR IGNORE INTO memory_settings
         (id, enabled, embedding_endpoint, embedding_model, embedding_dimensions,
          recent_message_limit, recall_depth, ranking_mode, reranker_model,
          reranker_candidate_limit, updated_at)
         VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            settings.enabled as i64,
            settings.embedding_endpoint,
            settings.embedding_model,
            settings.embedding_dimensions,
            settings.recent_message_limit,
            settings.recall_depth,
            settings.ranking_mode,
            settings.reranker_model,
            settings.reranker_candidate_limit,
            now_ms(),
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn get_settings(conn: &Connection) -> Result<MemorySettings, String> {
    conn.query_row(
        "SELECT enabled, embedding_endpoint, embedding_model, embedding_dimensions,
                recent_message_limit, recall_depth, ranking_mode, reranker_model,
                reranker_candidate_limit
         FROM memory_settings WHERE id = 1",
        [],
        |row| {
            Ok(MemorySettings {
                enabled: row.get::<_, i64>(0)? != 0,
                embedding_endpoint: row.get(1)?,
                embedding_model: row.get(2)?,
                embedding_dimensions: row.get(3)?,
                recent_message_limit: row.get(4)?,
                recall_depth: row.get(5)?,
                ranking_mode: row.get(6)?,
                reranker_model: row.get(7)?,
                reranker_candidate_limit: row.get(8)?,
            })
        },
    )
    .map_err(|e| e.to_string())
}

pub fn save_settings(conn: &Connection, settings: &MemorySettings) -> Result<(), String> {
    conn.execute(
        "UPDATE memory_settings
         SET enabled = ?1, embedding_endpoint = ?2, embedding_model = ?3,
             embedding_dimensions = ?4, recent_message_limit = ?5,
             recall_depth = ?6, ranking_mode = ?7, reranker_model = ?8,
             reranker_candidate_limit = ?9, updated_at = ?10
         WHERE id = 1",
        params![
            settings.enabled as i64,
            settings.embedding_endpoint,
            settings.embedding_model,
            settings.embedding_dimensions,
            settings.recent_message_limit,
            settings.recall_depth,
            settings.ranking_mode,
            settings.reranker_model,
            settings.reranker_candidate_limit,
            now_ms(),
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn insert_memory(conn: &Connection, input: NewMemoryInput) -> Result<Memory, String> {
    validate_embedding(&input.embedding, input.embedding_dimensions)?;
    validate_memory_input(&input)?;

    let id = new_id();
    let now = now_ms();
    let blob = encode_embedding(&input.embedding);
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    tx.execute(
        "INSERT INTO memories
         (id, conversation_id, character_id, persona_id, kind, content, embedding,
          embedding_dimensions, embedding_model, importance, pinned, status,
          created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 'active', ?12, ?12)",
        params![
            id,
            input.conversation_id,
            input.character_id,
            input.persona_id,
            input.kind,
            input.content,
            blob,
            input.embedding_dimensions,
            input.embedding_model,
            input.importance,
            input.pinned as i64,
            now,
        ],
    )
    .map_err(|e| e.to_string())?;

    for message_id in &input.source_message_ids {
        tx.execute(
            "INSERT OR IGNORE INTO memory_sources (memory_id, message_id) VALUES (?1, ?2)",
            params![id, message_id],
        )
        .map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())?;
    get_memory(conn, &id)?.ok_or_else(|| format!("Memory '{id}' was not created"))
}

pub fn get_memory(conn: &Connection, id: &str) -> Result<Option<Memory>, String> {
    conn.query_row(
        &format!("{SELECT_MEMORY} WHERE id = ?1"),
        params![id],
        row_to_memory,
    )
    .optional()
    .map_err(|e| e.to_string())
}

pub fn list_memories(conn: &Connection, conversation_id: &str) -> Result<Vec<Memory>, String> {
    let mut stmt = conn
        .prepare(&format!(
            "{SELECT_MEMORY} WHERE conversation_id = ?1 ORDER BY pinned DESC, updated_at DESC"
        ))
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![conversation_id], row_to_memory)
        .map_err(|e| e.to_string())?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| e.to_string())
}

pub fn list_active_candidates(
    conn: &Connection,
    conversation_id: &str,
    persona_id: Option<&str>,
    embedding_model: &str,
    dimensions: i64,
) -> Result<Vec<MemoryCandidate>, String> {
    let mut stmt = conn
        .prepare(&format!(
            "SELECT {MEMORY_COLUMNS}, embedding FROM memories
             WHERE conversation_id = ?1 AND persona_id IS ?2 AND status = 'active'
               AND embedding_model = ?3 AND embedding_dimensions = ?4"
        ))
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(
            params![conversation_id, persona_id, embedding_model, dimensions],
            |row| Ok((row_to_memory(row)?, row.get::<_, Vec<u8>>("embedding")?)),
        )
        .map_err(|e| e.to_string())?;

    rows.map(|row| {
        let (memory, blob) = row.map_err(|e| e.to_string())?;
        Ok(MemoryCandidate {
            embedding: decode_embedding(&blob, memory.embedding_dimensions)?,
            memory,
        })
    })
    .collect()
}

pub fn update_memory(
    conn: &Connection,
    id: &str,
    content: &str,
    embedding: &[f32],
    dimensions: i64,
    embedding_model: &str,
    importance: i64,
    pinned: bool,
) -> Result<(), String> {
    validate_embedding(embedding, dimensions)?;
    validate_importance(importance)?;
    if content.trim().is_empty() || embedding_model.trim().is_empty() {
        return Err("Memory content and embedding model are required".into());
    }
    conn.execute(
        "UPDATE memories SET content = ?2, embedding = ?3, embedding_dimensions = ?4,
             embedding_model = ?5, importance = ?6, pinned = ?7, status = 'active',
             updated_at = ?8 WHERE id = ?1",
        params![
            id,
            content.trim(),
            encode_embedding(embedding),
            dimensions,
            embedding_model,
            importance,
            pinned as i64,
            now_ms()
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn delete_memory(conn: &Connection, id: &str) -> Result<(), String> {
    conn.execute("DELETE FROM memories WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn clear_memories(conn: &Connection, conversation_id: &str) -> Result<(), String> {
    conn.execute(
        "DELETE FROM memories WHERE conversation_id = ?1",
        params![conversation_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn mark_stale_for_messages(conn: &Connection, message_ids: &[String]) -> Result<(), String> {
    set_status_for_messages(conn, message_ids, MemoryEntryStatus::Stale)
}

pub fn invalidate_for_messages(conn: &Connection, message_ids: &[String]) -> Result<(), String> {
    set_status_for_messages(conn, message_ids, MemoryEntryStatus::Invalid)
}

pub fn touch_recalled(conn: &Connection, memory_ids: &[String]) -> Result<(), String> {
    let now = now_ms();
    for id in memory_ids {
        conn.execute(
            "UPDATE memories SET last_recalled_at = ?2 WHERE id = ?1",
            params![id, now],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn set_status_for_messages(
    conn: &Connection,
    message_ids: &[String],
    status: MemoryEntryStatus,
) -> Result<(), String> {
    let now = now_ms();
    for message_id in message_ids {
        conn.execute(
            "UPDATE memories SET status = ?1, updated_at = ?2
             WHERE id IN (SELECT memory_id FROM memory_sources WHERE message_id = ?3)",
            params![status, now, message_id],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn encode_embedding(embedding: &[f32]) -> Vec<u8> {
    embedding
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect()
}

pub fn decode_embedding(blob: &[u8], dimensions: i64) -> Result<Vec<f32>, String> {
    let expected_bytes = usize::try_from(dimensions)
        .ok()
        .and_then(|dimensions| dimensions.checked_mul(std::mem::size_of::<f32>()))
        .ok_or_else(|| "Stored memory embedding has invalid dimensions".to_string())?;
    if blob.len() != expected_bytes {
        return Err("Stored memory embedding has an invalid byte length".into());
    }
    let vector: Vec<f32> = blob
        .chunks_exact(std::mem::size_of::<f32>())
        .map(|bytes| f32::from_le_bytes(bytes.try_into().expect("chunk size is four")))
        .collect();
    if vector.iter().any(|value| !value.is_finite()) {
        return Err("Stored memory embedding contains a non-finite value".into());
    }
    Ok(vector)
}

fn validate_memory_input(input: &NewMemoryInput) -> Result<(), String> {
    if input.content.trim().is_empty() || input.embedding_model.trim().is_empty() {
        return Err("Memory content and embedding model are required".into());
    }
    validate_importance(input.importance)
}

fn validate_embedding(embedding: &[f32], dimensions: i64) -> Result<(), String> {
    let expected_dimensions = usize::try_from(dimensions)
        .map_err(|_| "Memory embedding dimensions must be positive".to_string())?;
    if embedding.len() != expected_dimensions {
        return Err("Memory embedding dimensions do not match its vector length".into());
    }
    if embedding.iter().any(|value| !value.is_finite()) {
        return Err("Memory embedding contains a non-finite value".into());
    }
    Ok(())
}

fn validate_importance(importance: i64) -> Result<(), String> {
    if !(1..=5).contains(&importance) {
        return Err("Memory importance must be between 1 and 5".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedding_round_trips_as_little_endian_f32() {
        let values = vec![0.25, -1.5, 3.0];
        assert_eq!(
            decode_embedding(&encode_embedding(&values), 3).unwrap(),
            values
        );
    }

    #[test]
    fn embedding_rejects_invalid_length_and_non_finite_values() {
        assert!(decode_embedding(&[0; 3], 1).is_err());
        assert!(validate_embedding(&[f32::NAN], 1).is_err());
    }
}
