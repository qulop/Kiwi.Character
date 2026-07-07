//! Tauri commands — the contract the React frontend calls via `invoke`
//! (see `client/src/api.ts`). JS sends camelCase arg keys; Tauri maps them to
//! these snake_case parameters automatically.

use tauri::{AppHandle, Emitter, State};

use crate::models::{
    Character, ChatMessage, EndpointTestResult, HistoryItem, ModelSettings, NewCharacterInput,
};
use crate::openai::{self, ChatReqMsg};
use crate::state::{new_id, now_ms, AppState, Conversation, Store};

// ---- Characters ----------------------------------------------------------

#[tauri::command]
pub fn list_characters(state: State<'_, AppState>) -> Vec<Character> {
    state.store.lock().unwrap().characters.clone()
}

#[tauri::command]
pub fn get_character(id: String, state: State<'_, AppState>) -> Result<Character, String> {
    state
        .store
        .lock()
        .unwrap()
        .characters
        .iter()
        .find(|c| c.id == id)
        .cloned()
        .ok_or_else(|| format!("Character '{id}' not found"))
}

#[tauri::command]
pub fn create_character(input: NewCharacterInput, state: State<'_, AppState>) -> Character {
    let character = Character {
        id: new_id(),
        name: input.name,
        info: input.info,
        avatar: input.avatar,
        appearance: input.appearance,
        description: input.description,
        initial_message: input.initial_message,
    };
    state
        .store
        .lock()
        .unwrap()
        .characters
        .insert(0, character.clone());
    character
}

// ---- History / conversations --------------------------------------------

#[tauri::command]
pub fn list_history(state: State<'_, AppState>) -> Vec<HistoryItem> {
    let store = state.store.lock().unwrap();
    let mut items: Vec<HistoryItem> = store
        .conversations
        .values()
        .map(|conv| {
            let name = store
                .characters
                .iter()
                .find(|c| c.id == conv.character_id)
                .map(|c| c.name.clone())
                .unwrap_or_else(|| "Unknown".into());
            HistoryItem {
                id: conv.id.clone(),
                character_id: conv.character_id.clone(),
                name,
            }
        })
        .collect();
    items.sort_by(|a, b| a.name.cmp(&b.name));
    items
}

#[tauri::command]
pub fn list_messages(conversation_id: String, state: State<'_, AppState>) -> Vec<ChatMessage> {
    let mut store = state.store.lock().unwrap();
    // If we can't resolve a character for this id, just hand back an empty
    // thread rather than erroring — the UI tolerates that gracefully.
    if ensure_conversation(&mut store, &conversation_id).is_err() {
        return Vec::new();
    }
    store
        .conversations
        .get(&conversation_id)
        .map(|c| c.messages.clone())
        .unwrap_or_default()
}

#[tauri::command]
pub async fn send_message(
    conversation_id: String,
    content: String,
    state: State<'_, AppState>,
) -> Result<ChatMessage, String> {
    // 1. Record the user's message and snapshot everything we need for the
    //    request, then drop the lock before awaiting the network call.
    let (settings, character, history) = {
        let mut store = state.store.lock().unwrap();
        ensure_conversation(&mut store, &conversation_id)?;

        let user_msg = ChatMessage {
            id: new_id(),
            role: "user".into(),
            content,
            created_at: Some(now_ms()),
        };

        let conv = store
            .conversations
            .get_mut(&conversation_id)
            .expect("conversation ensured above");
        conv.messages.push(user_msg);

        let character_id = conv.character_id.clone();
        let history = conv.messages.clone();
        let settings = store.settings.clone();
        let character = store
            .characters
            .iter()
            .find(|c| c.id == character_id)
            .cloned()
            .ok_or_else(|| format!("Character '{character_id}' not found"))?;

        (settings, character, history)
    };

    // 2. Build the OpenAI request: persona system prompt + full thread.
    let mut req_msgs = vec![ChatReqMsg {
        role: "system".into(),
        content: build_system_prompt(&character, &settings),
    }];
    for m in &history {
        req_msgs.push(ChatReqMsg {
            role: m.role.clone(),
            content: m.content.clone(),
        });
    }

    // 3. Call the local LLM.
    let reply_text = openai::chat_completion(&settings, req_msgs).await?;

    // 4. Persist and return the assistant reply.
    let reply = ChatMessage {
        id: new_id(),
        role: "assistant".into(),
        content: reply_text,
        created_at: Some(now_ms()),
    };
    if let Some(conv) = state
        .store
        .lock()
        .unwrap()
        .conversations
        .get_mut(&conversation_id)
    {
        conv.messages.push(reply.clone());
    }
    Ok(reply)
}

/// Streaming counterpart to `send_message`. Emits one `chat://token` event per
/// content delta, then `chat://done` on success or `chat://error` on failure.
/// The user message is persisted before streaming; the assistant message after.
#[tauri::command]
pub async fn stream_message(
    conversation_id: String,
    content: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    // 1. Persist the user's message and snapshot everything we need for the
    //    request, then drop the lock before awaiting the network call.
    let (settings, character, history) = {
        let mut store = state.store.lock().unwrap();
        ensure_conversation(&mut store, &conversation_id)?;

        let user_msg = ChatMessage {
            id: new_id(),
            role: "user".into(),
            content,
            created_at: Some(now_ms()),
        };

        let conv = store
            .conversations
            .get_mut(&conversation_id)
            .expect("conversation ensured above");
        conv.messages.push(user_msg);

        let character_id = conv.character_id.clone();
        let history = conv.messages.clone();
        let settings = store.settings.clone();
        let character = store
            .characters
            .iter()
            .find(|c| c.id == character_id)
            .cloned()
            .ok_or_else(|| format!("Character '{character_id}' not found"))?;

        (settings, character, history)
    };

    // 2. Build the OpenAI request: persona system prompt + full thread.
    let mut req_msgs = vec![ChatReqMsg {
        role: "system".into(),
        content: build_system_prompt(&character, &settings),
    }];
    for m in &history {
        req_msgs.push(ChatReqMsg {
            role: m.role.clone(),
            content: m.content.clone(),
        });
    }

    // 3. Stream, emitting one event per token.
    let app_for_tokens = app.clone();
    let result = openai::chat_completion_stream(&settings, req_msgs, |tok| {
        let _ = app_for_tokens.emit("chat://token", tok);
    })
    .await;

    match result {
        Ok(full) => {
            // 4a. Persist the assistant reply, then signal completion.
            let reply = ChatMessage {
                id: new_id(),
                role: "assistant".into(),
                content: full,
                created_at: Some(now_ms()),
            };
            if let Some(conv) = state
                .store
                .lock()
                .unwrap()
                .conversations
                .get_mut(&conversation_id)
            {
                conv.messages.push(reply);
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
pub fn get_settings(state: State<'_, AppState>) -> ModelSettings {
    state.store.lock().unwrap().settings.clone()
}

#[tauri::command]
pub fn save_settings(settings: ModelSettings, state: State<'_, AppState>) {
    state.store.lock().unwrap().settings = settings;
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
pub fn load_model(settings: ModelSettings, state: State<'_, AppState>) {
    // LM Studio / Ollama load models on first use, so there's no standard
    // "load" call to make here. We just persist the chosen settings; the
    // selected model is used on the next `send_message`.
    state.store.lock().unwrap().settings = settings;
}

// ---- Helpers (not commands) ----------------------------------------------

/// Create the conversation lazily if it doesn't exist yet, seeding it with the
/// character's greeting. Conversation ids from the frontend look like
/// `conv-<characterId>` (see `App.tsx`).
fn ensure_conversation(store: &mut Store, conversation_id: &str) -> Result<(), String> {
    if store.conversations.contains_key(conversation_id) {
        return Ok(());
    }

    let character_id = conversation_id
        .strip_prefix("conv-")
        .unwrap_or(conversation_id)
        .to_string();

    let character = store
        .characters
        .iter()
        .find(|c| c.id == character_id)
        .cloned()
        .ok_or_else(|| format!("No character '{character_id}' for '{conversation_id}'"))?;

    let mut messages = Vec::new();
    if !character.initial_message.is_empty() {
        messages.push(ChatMessage {
            id: new_id(),
            role: "assistant".into(),
            content: character.initial_message.clone(),
            created_at: Some(now_ms()),
        });
    }

    store.conversations.insert(
        conversation_id.to_string(),
        Conversation {
            id: conversation_id.to_string(),
            character_id,
            messages,
        },
    );
    Ok(())
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
