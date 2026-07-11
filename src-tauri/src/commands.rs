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
    NewPersonaInput, Persona,
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
pub fn update_character(
    id: String,
    input: NewCharacterInput,
    state: State<'_, AppState>,
) -> Result<Character, String> {
    let db = state.db.lock().unwrap();
    let mut c = db::characters::update(&db.conn, &db.avatars_dir, &id, input)?;
    resolve_avatar(&db.avatars_dir, &mut c);
    Ok(c)
}

/// Live pre-check for the create/edit form: is this name free (case-insensitively)?
/// Pass `exclude_id` when editing so the character's own current name is allowed.
#[tauri::command]
pub fn character_name_available(
    name: String,
    exclude_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let db = state.db.lock().unwrap();
    let exclude = exclude_id.as_deref().unwrap_or("");
    Ok(!db::characters::name_exists(&db.conn, name.trim(), exclude)?)
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

// ---- Personas --------------------------------------------------------------

#[tauri::command]
pub fn list_personas(state: State<'_, AppState>) -> Result<Vec<Persona>, String> {
    let db = state.db.lock().unwrap();
    let mut personas = db::personas::list(&db.conn)?;
    for p in &mut personas {
        p.avatar = absolute_avatar(&db.avatars_dir, p.avatar.take());
    }
    Ok(personas)
}

#[tauri::command]
pub fn create_persona(
    input: NewPersonaInput,
    state: State<'_, AppState>,
) -> Result<Persona, String> {
    let db = state.db.lock().unwrap();
    let mut p = db::personas::insert(&db.conn, &db.avatars_dir, input)?;
    p.avatar = absolute_avatar(&db.avatars_dir, p.avatar.take());
    Ok(p)
}

#[tauri::command]
pub fn delete_persona(persona_id: String, state: State<'_, AppState>) -> Result<(), String> {
    let db = state.db.lock().unwrap();
    db::personas::delete(&db.conn, &db.avatars_dir, &persona_id)
}

// ---- History / conversations --------------------------------------------

#[tauri::command]
pub fn list_history(state: State<'_, AppState>) -> Result<Vec<HistoryItem>, String> {
    let db = state.db.lock().unwrap();
    let mut stmt = db
        .conn
        .prepare(
            "SELECT cv.id, cv.character_id, c.name, c.avatar_path,
                    COALESCE(cv.last_message_at, cv.created_at) AS ts
             FROM conversations cv
             JOIN characters c ON c.id = cv.character_id
             ORDER BY ts DESC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok(HistoryItem {
                id: r.get(0)?,
                character_id: r.get(1)?,
                name: r.get(2)?,
                avatar: r.get::<_, Option<String>>(3)?,
                last_message_at: r.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?;
    let mut items = rows
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| e.to_string())?;
    for h in &mut items {
        h.avatar = absolute_avatar(&db.avatars_dir, h.avatar.take());
    }
    Ok(items)
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
        db::messages::insert(&db.conn, &conversation_id, "user", &content, false)?;

        let character_id = db::conversations::character_id_of(&db.conn, &conversation_id)?;
        let character = db::characters::get(&db.conn, &character_id)?
            .ok_or_else(|| format!("Character '{character_id}' not found"))?;
        let history = db::messages::list_all(&db.conn, &conversation_id)?;
        let settings = db::settings::get(&db.conn)?;
        (settings, character, history)
    };

    // 2. Build the OpenAI request: persona system prompt + full thread.
    let req_msgs = build_request(&character, &settings, &history);

    // 3. Call the local LLM.
    let reply_text = openai::chat_completion(&settings, req_msgs).await?;
    if reply_text.trim().is_empty() {
        return Err("The model returned an empty response.".into());
    }

    // 4. Persist and return the assistant reply.
    let reply = {
        let db = state.db.lock().unwrap();
        db::messages::insert(&db.conn, &conversation_id, "assistant", &reply_text, false)?
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
        db::messages::insert(&db.conn, &conversation_id, "user", &content, false)?;

        let character_id = db::conversations::character_id_of(&db.conn, &conversation_id)?;
        let character = db::characters::get(&db.conn, &character_id)?
            .ok_or_else(|| format!("Character '{character_id}' not found"))?;
        let history = db::messages::list_all(&db.conn, &conversation_id)?;
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
            // 4a. Persist the assistant reply, then signal completion. Skip
            // storing an empty reply so it can't pollute future prompts.
            if full.trim().is_empty() {
                let _ = app.emit("chat://error", "The model returned an empty response.".to_string());
                return Ok(());
            }
            {
                let db = state.db.lock().unwrap();
                db::messages::insert(&db.conn, &conversation_id, "assistant", &full, false)?;
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

/// The hidden instruction sent as a "user" turn to make the model continue its
/// previous message.
const CONTINUE_PROMPT: &str =
    "(Continue your previous message from exactly where it left off. Do not repeat, \
     restart, greet, or acknowledge this instruction — simply continue the text seamlessly.)";

/// Empty-send / "continue". If the last turn is the assistant's, inject a hidden
/// technical user message so the model continues (never shown in the UI). If the
/// last turn is the user's (e.g. after a rewind/delete), just generate a reply
/// for it. Emits the same `chat://token|done|error` events as `stream_message`.
#[tauri::command]
pub async fn stream_continue(
    conversation_id: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let (settings, character, history) = {
        let db = state.db.lock().unwrap();
        db::conversations::ensure(&db.conn, &conversation_id)?;

        if db::messages::last_role(&db.conn, &conversation_id)?.as_deref() == Some("assistant") {
            db::messages::insert(&db.conn, &conversation_id, "user", CONTINUE_PROMPT, true)?;
        }

        let character_id = db::conversations::character_id_of(&db.conn, &conversation_id)?;
        let character = db::characters::get(&db.conn, &character_id)?
            .ok_or_else(|| format!("Character '{character_id}' not found"))?;
        let history = db::messages::list_all(&db.conn, &conversation_id)?;
        let settings = db::settings::get(&db.conn)?;
        (settings, character, history)
    };

    let req_msgs = build_request(&character, &settings, &history);

    let app_for_tokens = app.clone();
    let result = openai::chat_completion_stream(&settings, req_msgs, |tok| {
        let _ = app_for_tokens.emit("chat://token", tok);
    })
    .await;

    match result {
        Ok(full) => {
            if full.trim().is_empty() {
                let _ = app.emit("chat://error", "The model returned an empty response.".to_string());
                return Ok(());
            }
            {
                let db = state.db.lock().unwrap();
                db::messages::insert(&db.conn, &conversation_id, "assistant", &full, false)?;
            }
            let _ = app.emit("chat://done", ());
            Ok(())
        }
        Err(e) => {
            let _ = app.emit("chat://error", e.clone());
            Err(e)
        }
    }
}

#[tauri::command]
pub fn update_message(
    message_id: String,
    content: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.lock().unwrap();
    db::messages::update_content(&db.conn, &message_id, &content)
}

#[tauri::command]
pub fn delete_message(
    conversation_id: String,
    message_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.lock().unwrap();
    db::messages::delete(&db.conn, &conversation_id, &message_id)
}

/// Delete a whole conversation (chat history) but keep the character.
#[tauri::command]
pub fn delete_conversation(
    conversation_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.lock().unwrap();
    db::conversations::delete(&db.conn, &conversation_id)
}

/// Delete all messages positioned after `message_id` (rewind the thread to it).
#[tauri::command]
pub fn rewind_to_message(
    conversation_id: String,
    message_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.lock().unwrap();
    db::messages::rewind(&db.conn, &conversation_id, &message_id)
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

/// Models currently loaded on the server (LM Studio native API).
#[tauri::command]
pub async fn loaded_models(endpoint: String) -> Result<Vec<String>, String> {
    openai::loaded_models(&endpoint).await
}

/// Unload a model on the server via the `lms` CLI (LM Studio has no REST unload).
#[tauri::command]
pub async fn unload_model(model: String) -> Result<(), String> {
    if model.trim().is_empty() {
        return Err("No model to unload".into());
    }
    let joined = tauri::async_runtime::spawn_blocking(move || {
        std::process::Command::new("lms")
            .args(["unload", &model])
            .output()
    })
    .await
    .map_err(|e| e.to_string())?;

    let out = joined.map_err(|e| {
        format!("Could not run 'lms' (is LM Studio's CLI installed / on PATH?): {e}")
    })?;
    if !out.status.success() {
        return Err(format!(
            "lms unload failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(())
}

#[tauri::command]
pub async fn load_model(settings: ModelSettings, state: State<'_, AppState>) -> Result<(), String> {
    // Persist the choice first (so it sticks even if loading is slow or fails),
    // then ask the server to actually load the model. Drop the lock before await.
    {
        let db = state.db.lock().unwrap();
        db::settings::save(&db.conn, &settings)?;
    }
    openai::load_model(&settings).await
}

// ---- Helpers (not commands) ----------------------------------------------

/// Rewrite a stored relative avatar path (`avatars/<file>`) to an absolute
/// filesystem path the frontend turns into an asset URL via `convertFileSrc`.
fn absolute_avatar(avatars_dir: &Path, rel: Option<String>) -> Option<String> {
    rel.map(|r| {
        let file = r.strip_prefix("avatars/").unwrap_or(&r).to_string();
        avatars_dir.join(file).to_string_lossy().into_owned()
    })
}

fn resolve_avatar(avatars_dir: &Path, c: &mut Character) {
    c.avatar = absolute_avatar(avatars_dir, c.avatar.take());
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

    // Most chat templates (e.g. Qwen3) require the first non-system message to
    // be from the user and error otherwise ("No user query found in messages").
    // Our conversations open with a seeded assistant greeting, so skip any
    // leading assistant messages until the first user turn. The greeting still
    // lives in the DB and is shown in the UI — it's just not sent to the model.
    let mut seen_user = false;
    for m in history {
        if !seen_user {
            if m.role == "user" {
                seen_user = true;
            } else {
                continue;
            }
        }
        // Drop empty assistant turns (e.g. earlier blank replies from a reasoning
        // model) — feeding them back makes the model produce yet more empties.
        if m.role == "assistant" && m.content.trim().is_empty() {
            continue;
        }
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
