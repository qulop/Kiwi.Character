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

/// A user persona — who the user is presenting as in a chat.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Persona {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    /// Avatar image (data URL, file path, or asset URL).
    #[serde(default)]
    pub avatar: Option<String>,
    /// Reserved for a future "default persona" feature — always false for now.
    #[serde(default)]
    pub is_default: bool,
    #[serde(default)]
    pub created_at: i64,
}

/// Payload for creating a new persona (no id yet).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewPersonaInput {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub avatar: Option<String>,
}

/// A minimal character summary for a group's member list — just enough to
/// render an avatar + name without a second round trip.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupMemberBrief {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub avatar: Option<String>,
}

/// A group room — multiple characters (+ the user) in one chat. Group chat
/// itself isn't implemented yet; for now a group is created and listed.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Group {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub topic: String,
    #[serde(default)]
    pub background: String,
    /// Avatar image (data URL, file path, or asset URL). `None` means the UI
    /// should render a collage of the first members' avatars instead.
    #[serde(default)]
    pub avatar: Option<String>,
    /// All members, in the order they were added.
    #[serde(default)]
    pub members: Vec<GroupMemberBrief>,
    #[serde(default)]
    pub created_at: i64,
}

/// Payload for creating a new group (no id yet).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewGroupInput {
    pub name: String,
    #[serde(default)]
    pub topic: String,
    #[serde(default)]
    pub background: String,
    #[serde(default)]
    pub avatar: Option<String>,
    /// Member character ids, in the order the user picked them. Must have
    /// at least two.
    pub member_ids: Vec<String>,
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
    /// Character avatar (absolute path from the backend; asset URL on the JS side).
    #[serde(default)]
    pub avatar: Option<String>,
    /// ms epoch used to bucket the history by time (newest activity).
    #[serde(default)]
    pub last_message_at: i64,
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
