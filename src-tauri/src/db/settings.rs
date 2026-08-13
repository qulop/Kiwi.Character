//! `settings` singleton-row repository (`id = 1`).

use rusqlite::{params, Connection};

use crate::models::ModelSettings;

/// Ensure the single settings row exists, seeding defaults if not.
pub fn ensure_row(conn: &Connection) -> Result<(), String> {
    let has: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM settings WHERE id = 1)",
            [],
            |r| r.get::<_, i64>(0),
        )
        .map(|n| n == 1)
        .map_err(|e| e.to_string())?;
    if !has {
        save(conn, &ModelSettings::default())?;
    }
    return Ok(());
}

pub fn get(conn: &Connection) -> Result<ModelSettings, String> {
    return conn.query_row(
        "SELECT endpoint, model, context_length, gpu_offload, temperature, max_tokens, system_prompt
         FROM settings WHERE id = 1",
        [],
        |r| {
            Ok(ModelSettings {
                endpoint: r.get(0)?,
                model: r.get(1)?,
                context_length: r.get(2)?,
                gpu_offload: r.get(3)?,
                // SQLite REAL is f64; ModelSettings.temperature is f32.
                temperature: r.get::<_, f64>(4)? as f32,
                max_tokens: r.get(5)?,
                system_prompt: r.get(6)?,
            })
        },
    )
    .map_err(|e| e.to_string());
}

pub fn save(conn: &Connection, s: &ModelSettings) -> Result<(), String> {
    conn.execute(
        "INSERT INTO settings
           (id, endpoint, model, context_length, gpu_offload, temperature, max_tokens, system_prompt)
         VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(id) DO UPDATE SET
           endpoint = ?1, model = ?2, context_length = ?3, gpu_offload = ?4,
           temperature = ?5, max_tokens = ?6, system_prompt = ?7",
        params![
            s.endpoint,
            s.model,
            s.context_length,
            s.gpu_offload,
            s.temperature as f64,
            s.max_tokens,
            s.system_prompt
        ],
    )
    .map_err(|e| e.to_string())?;
    return Ok(());
}
