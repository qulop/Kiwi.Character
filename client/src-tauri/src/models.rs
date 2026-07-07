//! Domain types mirrored 1:1 with the frontend's `src/types.ts`.
//!
//! `serde(rename_all = "camelCase")` makes the JSON Tauri passes to/from the
//! React side line up with the TS interfaces (e.g. `initialMessage`,
//! `contextLength`) while keeping idiomatic snake_case in Rust.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Character {
    pub id: String,
    pub name: String,
    /// Short blurb shown under the name on cards.
    pub info: String,
    /// Avatar image (data URL, file path, or asset URL).
    #[serde(default)]
    pub avatar: Option<String>,
    #[serde(default)]
    pub appearance: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub initial_message: String,
    #[serde(default)]
    pub is_favorite: bool,
    /// ms epoch the character was created (drives the "Recent" category).
    #[serde(default)]
    pub created_at: i64,
    /// ms epoch of the newest message across the character's conversations,
    /// or `None` if never chatted with (also drives "Recent").
    #[serde(default)]
    pub last_message_at: Option<i64>,
}

/// Payload for creating a new character (no id yet).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewCharacterInput {
    pub name: String,
    pub info: String,
    #[serde(default)]
    pub appearance: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub initial_message: String,
    #[serde(default)]
    pub avatar: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessage {
    pub id: String,
    /// "user" | "assistant"
    pub role: String,
    pub content: String,
    /// ms epoch — handy for ordering / display.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<i64>,
}

/// A row in the sidebar "History" list.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryItem {
    /// Conversation id.
    pub id: String,
    pub character_id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelSettings {
    pub endpoint: String,
    pub model: String,
    pub context_length: i64,
    pub gpu_offload: i64,
    pub temperature: f32,
    pub max_tokens: i64,
    pub system_prompt: String,
}

impl Default for ModelSettings {
    /// Mirrors `DEFAULT_SETTINGS` in `types.ts`.
    fn default() -> Self {
        Self {
            endpoint: "http://localhost:1234/v1".into(),
            model: "llama-3.1-8b-instruct".into(),
            context_length: 100,
            gpu_offload: 60,
            temperature: 0.8,
            max_tokens: 2048,
            system_prompt: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EndpointTestResult {
    pub ok: bool,
    /// Models the endpoint reports as available.
    pub models: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}
