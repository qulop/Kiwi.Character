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
    Character, ChatMessage, EndpointTestResult, Group, HistoryItem, LoadedModel, MemorySettings, ModelLoadResult, ModelSettings,
    NewCharacterInput, NewGroupInput, NewPersonaInput, Persona,
};
use crate::openai::{self, ChatReqMsg};
use crate::prompts;
use crate::state::AppState;
use crate::notifications::{notify_user, NotificationKind};

// ---- Characters ----------------------------------------------------------

#[tauri::command]
pub fn list_characters(state: State<'_, AppState>) -> Result<Vec<Character>, String> {
    let db = state.db.lock().unwrap();
    let mut chars = db::characters::list(&db.conn)?;
    for c in &mut chars {
        resolve_avatar(&db.avatars_dir, c);
    }

    return Ok(chars);
}

#[tauri::command]
pub fn get_character(id: String, state: State<'_, AppState>) -> Result<Character, String> {
    let db = state.db.lock().unwrap();
    let mut c =
        db::characters::get(&db.conn, &id)?.ok_or_else(|| format!("Character '{id}' not found"))?;
    resolve_avatar(&db.avatars_dir, &mut c);
    return Ok(c);
}

#[tauri::command]
pub fn create_character(
    input: NewCharacterInput,
    state: State<'_, AppState>,
) -> Result<Character, String> {
    let db = state.db.lock().unwrap();
    let mut c = db::characters::insert(&db.conn, &db.avatars_dir, input)?;
    log::info!(target: "kiwi::characters", "Created character '{}' (id={})", c.name, c.id);
    resolve_avatar(&db.avatars_dir, &mut c);
    return Ok(c);
}

#[tauri::command]
pub fn update_character(
    id: String,
    input: NewCharacterInput,
    state: State<'_, AppState>,
) -> Result<Character, String> {
    let db = state.db.lock().unwrap();
    let mut c = db::characters::update(&db.conn, &db.avatars_dir, &id, input)?;
    log::info!(target: "kiwi::characters", "Updated character '{}' (id={id})", c.name);
    resolve_avatar(&db.avatars_dir, &mut c);
    return Ok(c);
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
    return Ok(!db::characters::name_exists(&db.conn, name.trim(), exclude)?);
}

#[tauri::command]
pub fn set_favorite(
    character_id: String,
    favorite: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.lock().unwrap();

    return db::characters::set_favorite(&db.conn, &character_id, favorite);
}

#[tauri::command]
pub fn set_visibility(
    character_id: String,
    visible: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.lock().unwrap();

    return db::characters::set_visibility(&db.conn, &character_id, visible);
}

#[tauri::command]
pub fn delete_character(character_id: String, state: State<'_, AppState>) -> Result<(), String> {
    let db = state.db.lock().unwrap();
    db::characters::delete(&db.conn, &db.avatars_dir, &character_id)?;
    log::info!(target: "kiwi::characters", "Deleted character id={character_id}");
    return Ok(());
}

// ---- Personas --------------------------------------------------------------

#[tauri::command]
pub fn list_personas(state: State<'_, AppState>) -> Result<Vec<Persona>, String> {
    let db = state.db.lock().unwrap();
    let mut personas = db::personas::list(&db.conn)?;
    for p in &mut personas {
        p.avatar = absolute_avatar(&db.avatars_dir, p.avatar.take());
    }
    return Ok(personas);
}

#[tauri::command]
pub fn create_persona(
    input: NewPersonaInput,
    state: State<'_, AppState>,
) -> Result<Persona, String> {
    let db = state.db.lock().unwrap();
    let mut p = db::personas::insert(&db.conn, &db.avatars_dir, input)?;
    log::info!(target: "kiwi::personas", "Created persona '{}' (id={})", p.name, p.id);
    p.avatar = absolute_avatar(&db.avatars_dir, p.avatar.take());
    return Ok(p);
}

#[tauri::command]
pub fn update_persona(
    id: String,
    input: NewPersonaInput,
    state: State<'_, AppState>,
) -> Result<Persona, String> {
    let db = state.db.lock().unwrap();
    let mut p = db::personas::update(&db.conn, &db.avatars_dir, &id, input)?;
    log::info!(target: "kiwi::personas", "Updated persona '{}' (id={id})", p.name);
    p.avatar = absolute_avatar(&db.avatars_dir, p.avatar.take());
    return Ok(p);
}

#[tauri::command]
pub fn delete_persona(persona_id: String, state: State<'_, AppState>) -> Result<(), String> {
    let db = state.db.lock().unwrap();
    db::personas::delete(&db.conn, &db.avatars_dir, &persona_id)?;
    log::info!(target: "kiwi::personas", "Deleted persona id={persona_id}");
    return Ok(());
}

/// The persona currently selected for this chat, if any (survives relaunch).
#[tauri::command]
pub fn get_active_persona(
    conversation_id: String,
    state: State<'_, AppState>,
) -> Result<Option<Persona>, String> {
    let db = state.db.lock().unwrap();
    let Some(pid) = db::conversations::active_persona_id(&db.conn, &conversation_id)? else {
        return Ok(None);
    };
    let mut p = db::personas::get(&db.conn, &pid)?;
    if let Some(p) = &mut p {
        p.avatar = absolute_avatar(&db.avatars_dir, p.avatar.take());
    }
    return Ok(p);
}

/// Select (or, with `None`, clear) the persona for this chat.
#[tauri::command]
pub fn set_active_persona(
    conversation_id: String,
    persona_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.lock().unwrap();
    db::conversations::ensure(&db.conn, &conversation_id)?;
    log::info!(
        target: "kiwi::personas",
        "Set active persona for conversation '{conversation_id}' to {persona_id:?}"
    );
    return db::conversations::set_active_persona(&db.conn, &conversation_id, persona_id.as_deref());
}

// ---- Groups ----------------------------------------------------------------

#[tauri::command]
pub fn list_groups(state: State<'_, AppState>) -> Result<Vec<Group>, String> {
    let db = state.db.lock().unwrap();
    let mut groups = db::groups::list(&db.conn)?;
    for g in &mut groups {
        g.avatar = absolute_avatar(&db.avatars_dir, g.avatar.take());
        for m in &mut g.members {
            m.avatar = absolute_avatar(&db.avatars_dir, m.avatar.take());
        }
    }
    return Ok(groups);
}

#[tauri::command]
pub fn create_group(input: NewGroupInput, state: State<'_, AppState>) -> Result<Group, String> {
    let db = state.db.lock().unwrap();
    let mut g = db::groups::insert(&db.conn, &db.avatars_dir, input)?;
    log::info!(
        target: "kiwi::groups",
        "Created group '{}' (id={}) with {} member(s)",
        g.name,
        g.id,
        g.members.len()
    );
    g.avatar = absolute_avatar(&db.avatars_dir, g.avatar.take());
    for m in &mut g.members {
        m.avatar = absolute_avatar(&db.avatars_dir, m.avatar.take());
    }
    return Ok(g);
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
    return Ok(items);
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
    return db::messages::list(&db.conn, &conversation_id);
}

#[tauri::command]
pub async fn send_message(
    conversation_id: String,
    content: String,
    state: State<'_, AppState>,
) -> Result<ChatMessage, String> {
    // 1. Persist the user message and snapshot context, then drop the lock.
    let (settings, character, persona, history) = {
        let db = state.db.lock().unwrap();
        db::conversations::ensure(&db.conn, &conversation_id)?;
        db::messages::insert(&db.conn, &conversation_id, "user", &content, false)?;

        let character_id = db::conversations::character_id_of(&db.conn, &conversation_id)?;
        let character = db::characters::get(&db.conn, &character_id)?
            .ok_or_else(|| format!("Character '{character_id}' not found"))?;
        let persona = active_persona(&db.conn, &conversation_id)?;
        let history = db::messages::list_all(&db.conn, &conversation_id)?;
        let settings = db::settings::get(&db.conn)?;
        (settings, character, persona, history)
    };

    // 2. Build the OpenAI request: persona system prompt + full thread.
    let req_msgs = build_request(&character, persona.as_ref(), &settings, &history);
    log_prompt(&conversation_id, &settings, &req_msgs);

    // 3. Call the local LLM.
    let reply_text = openai::chat_completion(&settings, req_msgs).await.map_err(|e| {
        log::error!(target: "kiwi::llm", "send_message failed for '{conversation_id}': {e}");
        e
    })?;
    if reply_text.trim().is_empty() {
        log::warn!(target: "kiwi::llm", "Empty reply for conversation '{conversation_id}'");
        return Err("The model returned an empty response.".into());
    }
    log::info!(
        target: "kiwi::llm",
        "Reply for conversation '{conversation_id}': {} chars",
        reply_text.chars().count()
    );

    // 4. Persist and return the assistant reply.
    let reply = {
        let db = state.db.lock().unwrap();
        db::messages::insert(&db.conn, &conversation_id, "assistant", &reply_text, false)?
    };
    return Ok(reply);
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
    let (settings, mut character, persona, history) = {
        let db = state.db.lock().unwrap();
        db::conversations::ensure(&db.conn, &conversation_id)?;
        db::messages::insert(&db.conn, &conversation_id, "user", &content, false)?;

        let character_id = db::conversations::character_id_of(&db.conn, &conversation_id)?;
        let character = db::characters::get(&db.conn, &character_id)?
            .ok_or_else(|| format!("Character '{character_id}' not found"))?;
        let persona = active_persona(&db.conn, &conversation_id)?;
        let history = db::messages::list_all(&db.conn, &conversation_id)?;
        let settings = db::settings::get(&db.conn)?;

        (settings, character, persona, history)
    };

    // 2. Build the request.
    let req_msgs = build_request(&character, persona.as_ref(), &settings, &history);
    log_prompt(&conversation_id, &settings, &req_msgs);

    // 3. Stream, emitting one event per token.
    let app_for_tokens = app.clone();
    let result = openai::chat_completion_stream(&settings, req_msgs, |tok| {
        let _ = app_for_tokens.emit("chat://token", tok);
    })
    .await;

    return match result {
        Ok(full) => {
            // 4a. Persist the assistant reply, then signal completion. Skip
            // storing an empty reply so it can't pollute future prompts.
            if full.trim().is_empty() {
                log::warn!(target: "kiwi::llm", "Empty streamed reply for conversation '{conversation_id}'");
                let _ = app.emit("chat://error", "The model returned an empty response.".to_string());
                return Ok(());
            }
            log::info!(
                target: "kiwi::llm",
                "Streamed reply for conversation '{conversation_id}': {} chars",
                full.chars().count()
            );
            {
                let db = state.db.lock().unwrap();
                db::messages::insert(&db.conn, &conversation_id, "assistant", &full, false)?;

                resolve_avatar(&db.avatars_dir, &mut character); // Resolve avatar for notification
            }
            let _ = app.emit("chat://done", ());

            return notify_user(app, NotificationKind::ChatDone(character, full));
        }
        Err(e) => {
            // 4b. Surface the real error to the UI.
            log::error!(target: "kiwi::llm", "stream_message failed for '{conversation_id}': {e}");
            let _ = app.emit("chat://error", e.clone());
            Err(e)
        }
    };
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
    let (settings, mut character, persona, history) = {
        let db = state.db.lock().unwrap();
        db::conversations::ensure(&db.conn, &conversation_id)?;

        if db::messages::last_role(&db.conn, &conversation_id)?.as_deref() == Some("assistant") {
            db::messages::insert(&db.conn, &conversation_id, "user", CONTINUE_PROMPT, true)?;
        }

        let character_id = db::conversations::character_id_of(&db.conn, &conversation_id)?;
        let character = db::characters::get(&db.conn, &character_id)?
            .ok_or_else(|| format!("Character '{character_id}' not found"))?;
        let persona = active_persona(&db.conn, &conversation_id)?;
        let history = db::messages::list_all(&db.conn, &conversation_id)?;
        let settings = db::settings::get(&db.conn)?;
        (settings, character, persona, history)
    };

    let req_msgs = build_request(&character, persona.as_ref(), &settings, &history);
    log_prompt(&conversation_id, &settings, &req_msgs);

    let app_for_tokens = app.clone();
    let result = openai::chat_completion_stream(&settings, req_msgs, |tok| {
        let _ = app_for_tokens.emit("chat://token", tok);
    })
    .await;

    return match result {
        Ok(full) => {
            if full.trim().is_empty() {
                log::warn!(target: "kiwi::llm", "Empty continued reply for conversation '{conversation_id}'");
                let _ = app.emit("chat://error", "The model returned an empty response.".to_string());
                return Ok(());
            }
            log::info!(
                target: "kiwi::llm",
                "Continued reply for conversation '{conversation_id}': {} chars",
                full.chars().count()
            );
            {
                let db = state.db.lock().unwrap();
                db::messages::insert(&db.conn, &conversation_id, "assistant", &full, false)?;

                resolve_avatar(&db.avatars_dir, &mut character); // Resolve avatar for notification
            }
            let _ = app.emit("chat://done", ());

            return notify_user(app, NotificationKind::ChatDone(character, full));
        }
        Err(e) => {
            log::error!(target: "kiwi::llm", "stream_continue failed for '{conversation_id}': {e}");
            let _ = app.emit("chat://error", e.clone());
            Err(e)
        }
    };
}

#[tauri::command]
pub fn update_message(
    message_id: String,
    content: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.lock().unwrap();
    return db::messages::update_content(&db.conn, &message_id, &content);
}

#[tauri::command]
pub fn delete_message(
    conversation_id: String,
    message_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.lock().unwrap();
    return db::messages::delete(&db.conn, &conversation_id, &message_id);
}

/// Delete a whole conversation (chat history) but keep the character.
#[tauri::command]
pub fn delete_conversation(
    conversation_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.lock().unwrap();
    return db::conversations::delete(&db.conn, &conversation_id);
}

/// Delete all messages positioned after `message_id` (rewind the thread to it).
#[tauri::command]
pub fn rewind_to_message(
    conversation_id: String,
    message_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.lock().unwrap();
    return db::messages::rewind(&db.conn, &conversation_id, &message_id);
}

// ---- Settings / model ----------------------------------------------------

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Result<ModelSettings, String> {
    let db = state.db.lock().unwrap();
    return db::settings::get(&db.conn);
}

#[tauri::command]
pub fn save_settings(settings: ModelSettings, state: State<'_, AppState>) -> Result<(), String> {
    let db = state.db.lock().unwrap();
    return db::settings::save(&db.conn, &settings);
}

#[tauri::command]
pub fn get_memory_settings(state: State<'_, AppState>) -> Result<MemorySettings, String> {
    let db = state.db.lock().unwrap();
    db::memories::get_settings(&db.conn)
}

#[tauri::command]
pub fn save_memory_settings(settings: MemorySettings, state: State<'_, AppState>) -> Result<(), String> {
    let db = state.db.lock().unwrap();
    db::memories::save_settings(&db.conn, &settings)
}

#[tauri::command]
pub async fn test_endpoint(endpoint: String) -> EndpointTestResult {
    return match openai::list_models(&endpoint).await {
        Ok(models) => {
            log::info!(target: "kiwi::llm", "Endpoint '{endpoint}' ok, {} model(s)", models.len());
            EndpointTestResult { ok: true, models, error: None }
        }
        Err(error) => {
            log::warn!(target: "kiwi::llm", "Endpoint '{endpoint}' test failed: {error}");
            EndpointTestResult { ok: false, models: Vec::new(), error: Some(error) }
        }
    };
}

/// Models currently loaded on the server (LM Studio native API).
#[tauri::command]
pub async fn loaded_models(endpoint: String) -> Result<Vec<LoadedModel>, String> {
    return openai::loaded_models(&endpoint).await;
}

#[tauri::command]
pub async fn load_embedding_model(endpoint: String, model: String) -> Result<ModelLoadResult, String> {
    return openai::load_auxiliary_model(&endpoint, &model).await;
}

/// Unload a model on the server via the `lms` CLI (LM Studio has no REST unload).
#[tauri::command]
pub async fn unload_model(model: String) -> Result<(), String> {
    if model.trim().is_empty() {
        return Err("No model to unload".into());
    }
    log::info!(target: "kiwi::llm", "Unloading model '{model}'");
    let joined = tauri::async_runtime::spawn_blocking({
        let model = model.clone();
        move || std::process::Command::new("lms").args(["unload", &model]).output()
    })
    .await
    .map_err(|e| e.to_string())?;

    let out = joined.map_err(|e| {
        format!("Could not run 'lms' (is LM Studio's CLI installed / on PATH?): {e}")
    })?;
    if !out.status.success() {
        let msg = String::from_utf8_lossy(&out.stderr).trim().to_string();
        log::error!(target: "kiwi::llm", "Unload of '{model}' failed: {msg}");
        return Err(format!("lms unload failed: {msg}"));
    }
    return Ok(());
}

#[tauri::command]
pub async fn load_model(
    settings: ModelSettings,
    state: State<'_, AppState>,
) -> Result<ModelLoadResult, String> {
    // Persist the choice first (so it sticks even if loading is slow or fails),
    // then ask the server to actually load the model. Drop the lock before await.
    {
        let db = state.db.lock().unwrap();
        db::settings::save(&db.conn, &settings)?;
    }
    log::info!(
        target: "kiwi::llm",
        "Loading model '{}' on '{}' with {}k context",
        settings.model,
        settings.endpoint,
        settings.context_length
    );
    let result = openai::load_model(&settings).await;
    if let Ok(load) = &result {
        log::info!(
            target: "kiwi::llm",
            "LM Studio loaded '{}' with reported context {:?}",
            settings.model,
            load.context_length
        );
    }
    if let Err(e) = &result {
        log::error!(target: "kiwi::llm", "Loading '{}' failed: {e}", settings.model);
    }
    return result;
}

// ---- Helpers (not commands) ----------------------------------------------

/// Rewrite a stored relative avatar path (`avatars/<file>`) to an absolute
/// filesystem path the frontend turns into an asset URL via `convertFileSrc`.
fn absolute_avatar(avatars_dir: &Path, rel: Option<String>) -> Option<String> {
    return rel.map(|r| {
        let file = r.strip_prefix("avatars/").unwrap_or(&r).to_string();
        avatars_dir.join(file).to_string_lossy().into_owned()
    });
}

fn resolve_avatar(avatars_dir: &Path, c: &mut Character) {
    c.avatar = absolute_avatar(avatars_dir, c.avatar.take());
}

/// Look up the persona selected for this chat, if any.
fn active_persona(
    conn: &rusqlite::Connection,
    conversation_id: &str,
) -> Result<Option<Persona>, String> {
    return match db::conversations::active_persona_id(conn, conversation_id)? {
        Some(pid) => db::personas::get(conn, &pid),
        None => Ok(None),
    };
}

/// Build the OpenAI `messages` array: a structured roleplay system prompt
/// followed by the stored conversation thread.
fn build_request(
    character: &Character,
    persona: Option<&Persona>,
    settings: &ModelSettings,
    history: &[ChatMessage],
) -> Vec<ChatReqMsg> {
    let first_user_index = history.iter().position(|m| m.role == "user");
    let opening_message = first_user_index.and_then(|index| {
        history[index + 1..]
            .iter()
            .all(|message| message.role != "user")
            .then(|| {
                history[..index]
                    .iter()
                    .find(|message| {
                        message.role == "assistant" && !message.content.trim().is_empty()
                    })
                    .map(|message| message.content.as_str())
            })
            .flatten()
    });

    let mut req_msgs = vec![ChatReqMsg {
        role: "system".into(),
        content: build_system_prompt(character, persona, settings, opening_message),
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
    return req_msgs;
}

/// Compose the system prompt that puts the model in character.
///
/// The rules intentionally come before the reference data. Character fields
/// are private context, not a checklist to quote back: a strong trait should
/// influence a response only when it naturally fits the current conversation.
fn build_system_prompt(
    c: &Character,
    persona: Option<&Persona>,
    settings: &ModelSettings,
    opening_message: Option<&str>,
) -> String {
    let character_block = build_character_block(c);
    let persona_block = persona.map(build_persona_block).unwrap_or_default();
    let additional_user_instructions = (!settings.system_prompt.trim().is_empty())
        .then(|| prompts::additional_user_instructions(&xml_escape(settings.system_prompt.trim())))
        .unwrap_or_default();
    let opening_context = opening_message
        .filter(|message| !message.trim().is_empty())
        .map(|message| prompts::opening_context(&xml_escape(message)))
        .unwrap_or_default();

    return prompts::system_prompt(
        &character_block,
        &persona_block,
        &additional_user_instructions,
        &opening_context,
    );
}

/// Render character profile data as inert XML reference material. Escaping
/// every field prevents profile text from being mistaken for prompt structure.
fn build_character_block(c: &Character) -> String {
    let name = xml_escape(&c.name);
    let short_info = xml_escape(&c.info);
    let appearance = xml_escape(&c.appearance);
    let description = xml_escape(&c.description);
    return prompts::character_block(&name, &short_info, &appearance, &description);
}

/// Render persona data as a private XML reference block.
fn build_persona_block(p: &Persona) -> String {
    let name = xml_escape(&p.name);
    let description = xml_escape(&p.description);
    return prompts::persona_block(&name, &description);
}

/// Escape the characters that would otherwise turn profile text into XML.
fn xml_escape(s: &str) -> String {
    return s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
}

/// Log the exact prompt sent to the model — the same messages array the model
/// receives, in full, for every conversation turn.
fn log_prompt(conversation_id: &str, settings: &ModelSettings, req_msgs: &[ChatReqMsg]) {
    let rendered = req_msgs
        .iter()
        .map(|m| format!("--- {} ---\n{}", m.role, m.content))
        .collect::<Vec<_>>()
        .join("\n\n");
    log::info!(
        target: "kiwi::prompt",
        "Prompt for conversation '{conversation_id}' (model '{}'):\n{rendered}",
        settings.model,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_character() -> Character {
        return Character {
            id: "c1".into(),
            name: "Aria".into(),
            info: String::new(),
            avatar: None,
            appearance: String::new(),
            description: String::new(),
            initial_message: String::new(),
            is_favorite: false,
            created_at: 0,
            last_message_at: None,
        };
    }

    #[test]
    fn system_prompt_omits_persona_block_when_none() {
        let p = build_system_prompt(&test_character(), None, &ModelSettings::default(), None);
        assert!(p.contains("<kiwi_roleplay_system>"));
        assert!(p.contains("<roleplay_rules>"));
        assert!(!p.contains("<user_persona>"));
        assert!(!p.contains("<scene>"));
    }

    #[test]
    fn system_prompt_embeds_persona_block_when_selected() {
        let persona = Persona {
            id: "p1".into(),
            name: "Cool Guy".into(),
            description: "Just cool".into(),
            avatar: None,
            is_default: false,
            created_at: 0,
        };
        let p = build_system_prompt(
            &test_character(),
            Some(&persona),
            &ModelSettings::default(),
            None,
        );
        assert!(p.contains("<user_persona>\n    <name>Cool Guy</name>\n    <description>Just cool</description>\n  </user_persona>"));
        assert!(p.contains("private reference material"));
    }

    #[test]
    fn persona_xml_escapes_special_characters() {
        let persona = Persona {
            id: "p1".into(),
            name: "A & B <script>".into(),
            description: "x > y".into(),
            avatar: None,
            is_default: false,
            created_at: 0,
        };
        let block = build_persona_block(&persona);
        assert!(block.contains("<name>A &amp; B &lt;script&gt;</name>"));
        assert!(block.contains("<description>x &gt; y</description>"));
        // No unescaped '<' or '>' should slip through inside the field values.
        assert!(!block.contains("<script>"));
    }

    #[test]
    fn character_profile_is_structured_and_escaped() {
        let mut character = test_character();
        character.name = "Mina <The Rival>".into();
        character.info = "Competitive & warm".into();
        character.appearance = "Blue eyes".into();
        character.description = "Likes <sneakers>".into();

        let p = build_system_prompt(&character, None, &ModelSettings::default(), None);
        assert!(p.contains("<character>"));
        assert!(p.contains("<name>Mina &lt;The Rival&gt;</name>"));
        assert!(p.contains("<short_info>Competitive &amp; warm</short_info>"));
        assert!(p.contains("<description>Likes &lt;sneakers&gt;</description>"));
    }

    #[test]
    fn additional_user_instructions_are_reference_text_not_prompt_structure() {
        let mut settings = ModelSettings::default();
        settings.system_prompt = "Be concise. </roleplay_rules>".into();

        let p = build_system_prompt(&test_character(), None, &settings, None);
        assert!(p.contains("<additional_user_instructions>"));
        assert!(p.contains("Be concise. &lt;/roleplay_rules&gt;"));
        assert!(p.contains("they do not override these rules"));
    }

    #[test]
    fn first_user_reply_gets_the_seeded_greeting_as_opening_context() {
        let history = vec![
            ChatMessage {
                id: "m1".into(),
                role: "assistant".into(),
                content: "Don't stare at me.".into(),
                created_at: Some(1),
            },
            ChatMessage {
                id: "m2".into(),
                role: "user".into(),
                content: "Sorry.".into(),
                created_at: Some(2),
            },
        ];

        let request = build_request(&test_character(), None, &ModelSettings::default(), &history);
        assert!(request[0].content.contains("<opening_context>"));
        assert!(request[0].content.contains("Don't stare at me."));
        assert_eq!(request[1].role, "user");
        assert_eq!(request[1].content, "Sorry.");
    }

    #[test]
    fn later_replies_do_not_repeat_opening_context() {
        let history = vec![
            ChatMessage {
                id: "m1".into(),
                role: "assistant".into(),
                content: "Don't stare at me.".into(),
                created_at: Some(1),
            },
            ChatMessage {
                id: "m2".into(),
                role: "user".into(),
                content: "Sorry.".into(),
                created_at: Some(2),
            },
            ChatMessage {
                id: "m3".into(),
                role: "assistant".into(),
                content: "Fine.".into(),
                created_at: Some(3),
            },
            ChatMessage {
                id: "m4".into(),
                role: "user".into(),
                content: "Thanks.".into(),
                created_at: Some(4),
            },
        ];

        let request = build_request(&test_character(), None, &ModelSettings::default(), &history);
        assert!(!request[0].content.contains("<opening_context>"));
        assert_eq!(request.len(), 4);
    }
}
