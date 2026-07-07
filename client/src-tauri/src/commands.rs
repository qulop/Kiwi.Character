//! Tauri commands — the contract the React frontend calls via `invoke`
//! (see `client/src/api.ts`). JS sends camelCase arg keys; Tauri maps them to
//! these snake_case parameters automatically.
//!
//! All persistence goes through the `db` repositories. The golden rule: never
//! hold the DB `MutexGuard` across an `.await` — snapshot, drop, await, re-lock.

use std::path::Path;

use tauri::{AppHandle, Emitter, State};

use crate::db;
use crate::models::{
    Character, ChatMessage, EndpointTestResult, HistoryItem, ModelSettings, NewCharacterInput,
};
use crate::openai::{self, ChatReqMsg};
use crate::state::AppState;

// ---- Characters ----------------------------------------------------------

#[tauri::command]
pub fn list_characters(state: State<'_, AppState>) -> Result<Vec<Character>, String> {
    let db = state.db.lock().unwrap();
    let mut chars = db::characters::list(&db.conn)?;
    for c in &mut chars {
        resolve_avatar(&db.avatars_dir, c);
    }
    Ok(chars)
}

#[tauri::command]
pub fn get_character(id: String, state: State<'_, AppState>) -> Result<Character, String> {
    let db = state.db.lock().unwrap();
    let mut c =
        db::characters::get(&db.conn, &id)?.ok_or_else(|| format!("Character '{id}' not found"))?;
    resolve_avatar(&db.avatars_dir, &mut c);
    Ok(c)
}

#[tauri::command]
pub fn create_character(
    input: NewCharacterInput,
    state: State<'_, AppState>,
) -> Result<Character, String> {
    let db = state.db.lock().unwrap();
    let mut c = db::characters::insert(&db.conn, &db.avatars_dir, input)?;
    resolve_avatar(&db.avatars_dir, &mut c);
    Ok(c)
}

#[tauri::command]
pub fn set_favorite(
    character_id: String,
    favorite: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.lock().unwrap();
    db::characters::set_favorite(&db.conn, &character_id, favorite)
}

#[tauri::command]
pub fn delete_character(character_id: String, state: State<'_, AppState>) -> Result<(), String> {
    let db = state.db.lock().unwrap();
    db::characters::delete(&db.conn, &db.avatars_dir, &character_id)
}

// ---- History / conversations --------------------------------------------

#[tauri::command]
pub fn list_history(state: State<'_, AppState>) -> Result<Vec<HistoryItem>, String> {
    let db = state.db.lock().unwrap();
    let mut stmt = db
        .conn
        .prepare(
            "SELECT cv.id, cv.character_id, c.name
             FROM conversations cv
             JOIN characters c ON c.id = cv.character_id
             ORDER BY COALESCE(cv.last_message_at, cv.created_at) DESC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok(HistoryItem {
                id: r.get(0)?,
                character_id: r.get(1)?,
                name: r.get(2)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_messages(
    conversation_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<ChatMessage>, String> {
    let db = state.db.lock().unwrap();
    // If the conversation can't be resolved (unknown character), return an empty
    // thread rather than erroring — the UI tolerates that gracefully.
    if db::conversations::ensure(&db.conn, &conversation_id).is_err() {
        return Ok(Vec::new());
    }
    db::messages::list(&db.conn, &conversation_id)
}

#[tauri::command]
pub async fn send_message(
    conversation_id: String,
    content: String,
    state: State<'_, AppState>,
) -> Result<ChatMessage, String> {
    // 1. Persist the user message and snapshot context, then drop the lock.
    let (settings, character, history) = {
        let db = state.db.lock().unwrap();
        db::conversations::ensure(&db.conn, &conversation_id)?;
        db::messages::insert(&db.conn, &conversation_id, "user", &content)?;

        let character_id = db::conversations::character_id_of(&db.conn, &conversation_id)?;
        let character = db::characters::get(&db.conn, &character_id)?
            .ok_or_else(|| format!("Character '{character_id}' not found"))?;
        let history = db::messages::list(&db.conn, &conversation_id)?;
        let settings = db::settings::get(&db.conn)?;
        (settings, character, history)
    };

    // 2. Build the OpenAI request: persona system prompt + full thread.
    let req_msgs = build_request(&character, &settings, &history);

    // 3. Call the local LLM.
    let reply_text = openai::chat_completion(&settings, req_msgs).await?;

    // 4. Persist and return the assistant reply.
    let reply = {
        let db = state.db.lock().unwrap();
        db::messages::insert(&db.conn, &conversation_id, "assistant", &reply_text)?
    };
    Ok(reply)
}

/// Streaming counterpart to `send_message`. Emits one `chat://token` event per
/// content delta, then `chat://done` on success or `chat://error` on failure.
#[tauri::command]
pub async fn stream_message(
    conversation_id: String,
    content: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    // 1. Persist the user message and snapshot context, then drop the lock.
    let (settings, character, history) = {
        let db = state.db.lock().unwrap();
        db::conversations::ensure(&db.conn, &conversation_id)?;
        db::messages::insert(&db.conn, &conversation_id, "user", &content)?;

        let character_id = db::conversations::character_id_of(&db.conn, &conversation_id)?;
        let character = db::characters::get(&db.conn, &character_id)?
            .ok_or_else(|| format!("Character '{character_id}' not found"))?;
        let history = db::messages::list(&db.conn, &conversation_id)?;
        let settings = db::settings::get(&db.conn)?;
        (settings, character, history)
    };

    // 2. Build the request.
    let req_msgs = build_request(&character, &settings, &history);

    // 3. Stream, emitting one event per token.
    let app_for_tokens = app.clone();
    let result = openai::chat_completion_stream(&settings, req_msgs, |tok| {
        let _ = app_for_tokens.emit("chat://token", tok);
    })
    .await;

    match result {
        Ok(full) => {
            // 4a. Persist the assistant reply, then signal completion.
            {
                let db = state.db.lock().unwrap();
                db::messages::insert(&db.conn, &conversation_id, "assistant", &full)?;
            }
            let _ = app.emit("chat://done", ());
            Ok(())
        }
        Err(e) => {
            // 4b. Surface the real error to the UI.
            let _ = app.emit("chat://error", e.clone());
            Err(e)
        }
    }
}

// ---- Settings / model ----------------------------------------------------

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Result<ModelSettings, String> {
    let db = state.db.lock().unwrap();
    db::settings::get(&db.conn)
}

#[tauri::command]
pub fn save_settings(settings: ModelSettings, state: State<'_, AppState>) -> Result<(), String> {
    let db = state.db.lock().unwrap();
    db::settings::save(&db.conn, &settings)
}

#[tauri::command]
pub async fn test_endpoint(endpoint: String) -> EndpointTestResult {
    match openai::list_models(&endpoint).await {
        Ok(models) => EndpointTestResult {
            ok: true,
            models,
            error: None,
        },
        Err(error) => EndpointTestResult {
            ok: false,
            models: Vec::new(),
            error: Some(error),
        },
    }
}

#[tauri::command]
pub fn load_model(settings: ModelSettings, state: State<'_, AppState>) -> Result<(), String> {
    // LM Studio / Ollama load models on first use, so there's no standard "load"
    // call. We just persist the chosen settings; the selected model is used on
    // the next send/stream.
    let db = state.db.lock().unwrap();
    db::settings::save(&db.conn, &settings)
}

// ---- Helpers (not commands) ----------------------------------------------

/// Rewrite a stored relative avatar path (`avatars/<file>`) to an absolute
/// filesystem path the frontend turns into an asset URL via `convertFileSrc`.
fn resolve_avatar(avatars_dir: &Path, c: &mut Character) {
    if let Some(rel) = c.avatar.take() {
        let file = rel.strip_prefix("avatars/").unwrap_or(&rel);
        c.avatar = Some(avatars_dir.join(file).to_string_lossy().into_owned());
    }
}

/// Build the OpenAI `messages` array: a persona system prompt followed by the
/// stored conversation thread.
fn build_request(
    character: &Character,
    settings: &ModelSettings,
    history: &[ChatMessage],
) -> Vec<ChatReqMsg> {
    let mut req_msgs = vec![ChatReqMsg {
        role: "system".into(),
        content: build_system_prompt(character, settings),
    }];
    for m in history {
        req_msgs.push(ChatReqMsg {
            role: m.role.clone(),
            content: m.content.clone(),
        });
    }
    req_msgs
}

/// Compose the system prompt that puts the model in character.
fn build_system_prompt(c: &Character, settings: &ModelSettings) -> String {
    let mut p = format!("You are {}.", c.name);
    if !c.info.is_empty() {
        p.push(' ');
        p.push_str(&c.info);
    }
    if !c.description.is_empty() {
        p.push_str("\n\nBackground:\n");
        p.push_str(&c.description);
    }
    if !c.appearance.is_empty() {
        p.push_str("\n\nAppearance:\n");
        p.push_str(&c.appearance);
    }
    p.push_str(
        "\n\nStay in character at all times. Speak in the first person as this character, \
         and never mention that you are an AI or language model.",
    );
    if !settings.system_prompt.trim().is_empty() {
        p.push_str("\n\n");
        p.push_str(settings.system_prompt.trim());
    }
    p
}
