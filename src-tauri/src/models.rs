//! Domain types mirrored 1:1 with the frontend's `src/types.ts`.
//!
//! `serde(rename_all = "camelCase")` makes the JSON Tauri passes to/from the
//! React side line up with the TS interfaces (e.g. `initialMessage`,
//! `contextLength`) while keeping idiomatic snake_case in Rust.

use std::{fmt, str::FromStr};

use rusqlite::{
    types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef},
    Result as SqlResult,
};
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

/// Result of asking LM Studio to load a model through its native API.
/// `context_length` is the effective token limit echoed back by LM Studio.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelLoadResult {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_length: Option<i64>,
}

impl Default for ModelSettings {
    /// Mirrors `DEFAULT_SETTINGS` in `types.ts`.
    fn default() -> Self {
        return Self {
            endpoint: "http://localhost:1234/v1".into(),
            model: "llama-3.1-8b-instruct".into(),
            context_length: 100,
            gpu_offload: 60,
            temperature: 0.8,
            max_tokens: 2048,
            system_prompt: String::new(),
        };
    }
}

/// Persisted configuration for long-term character memory.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemorySettings {
    pub enabled: bool,
    pub embedding_endpoint: String,
    pub embedding_model: String,
    pub embedding_dimensions: i64,
    pub recent_message_limit: i64,
    pub recall_depth: i64,
    pub ranking_mode: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reranker_model: Option<String>,
    pub reranker_candidate_limit: i64,
}

impl Default for MemorySettings {
    fn default() -> Self {
        Self {
            enabled: true,
            embedding_endpoint: "http://localhost:1234/v1".into(),
            embedding_model: "Qwen/Qwen3-Embedding-0.6B".into(),
            embedding_dimensions: 1024,
            recent_message_limit: 20,
            recall_depth: 6,
            ranking_mode: "embedding".into(),
            reranker_model: None,
            reranker_candidate_limit: 24,
        }
    }
}

/// The semantic category of a durable memory. Values are stored in SQLite as
/// lowercase identifiers and intentionally match the schema CHECK constraint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MemoryKind {
    Fact,
    Preference,
    Event,
    Relationship,
    Summary,
    Manual,
}

impl MemoryKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Fact => "fact",
            Self::Preference => "preference",
            Self::Event => "event",
            Self::Relationship => "relationship",
            Self::Summary => "summary",
            Self::Manual => "manual",
        }
    }
}

impl fmt::Display for MemoryKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for MemoryKind {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "fact" => Ok(Self::Fact),
            "preference" => Ok(Self::Preference),
            "event" => Ok(Self::Event),
            "relationship" => Ok(Self::Relationship),
            "summary" => Ok(Self::Summary),
            "manual" => Ok(Self::Manual),
            _ => Err(()),
        }
    }
}

impl ToSql for MemoryKind {
    fn to_sql(&self) -> SqlResult<ToSqlOutput<'_>> {
        Ok(self.as_str().into())
    }
}

impl FromSql for MemoryKind {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        match value {
            ValueRef::Text(b"fact") => Ok(Self::Fact),
            ValueRef::Text(b"preference") => Ok(Self::Preference),
            ValueRef::Text(b"event") => Ok(Self::Event),
            ValueRef::Text(b"relationship") => Ok(Self::Relationship),
            ValueRef::Text(b"summary") => Ok(Self::Summary),
            ValueRef::Text(b"manual") => Ok(Self::Manual),
            _ => Err(FromSqlError::InvalidType),
        }
    }
}

/// Lifecycle state of a memory entry. Only active entries are retrievable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MemoryEntryStatus {
    Active,
    Stale,
    Invalid,
}

impl MemoryEntryStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Stale => "stale",
            Self::Invalid => "invalid",
        }
    }
}

impl fmt::Display for MemoryEntryStatus {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for MemoryEntryStatus {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "active" => Ok(Self::Active),
            "stale" => Ok(Self::Stale),
            "invalid" => Ok(Self::Invalid),
            _ => Err(()),
        }
    }
}

impl ToSql for MemoryEntryStatus {
    fn to_sql(&self) -> SqlResult<ToSqlOutput<'_>> {
        Ok(self.as_str().into())
    }
}

impl FromSql for MemoryEntryStatus {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        match value {
            ValueRef::Text(b"active") => Ok(Self::Active),
            ValueRef::Text(b"stale") => Ok(Self::Stale),
            ValueRef::Text(b"invalid") => Ok(Self::Invalid),
            _ => Err(FromSqlError::InvalidType),
        }
    }
}

#[cfg(test)]
mod memory_enum_tests {
    use super::*;

    #[test]
    fn memory_enums_use_stable_sql_identifiers() {
        assert_eq!(MemoryKind::Preference.to_string(), "preference");
        assert_eq!("manual".parse(), Ok(MemoryKind::Manual));
        assert_eq!(MemoryEntryStatus::Stale.to_string(), "stale");
        assert_eq!("invalid".parse(), Ok(MemoryEntryStatus::Invalid));
    }
}

/// A durable memory scoped to one character conversation and optional persona.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Memory {
    pub id: String,
    pub conversation_id: String,
    pub character_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub persona_id: Option<String>,
    pub kind: MemoryKind,
    pub content: String,
    pub embedding_dimensions: i64,
    pub embedding_model: String,
    pub importance: i64,
    pub pinned: bool,
    pub status: MemoryEntryStatus,
    pub created_at: i64,
    pub updated_at: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_recalled_at: Option<i64>,
}

/// Repository-only insertion data. Later steps will expose narrower Tauri inputs.
#[derive(Debug, Clone)]
pub struct NewMemoryInput {
    pub conversation_id: String,
    pub character_id: String,
    pub persona_id: Option<String>,
    pub kind: MemoryKind,
    pub content: String,
    pub embedding: Vec<f32>,
    pub embedding_dimensions: i64,
    pub embedding_model: String,
    pub importance: i64,
    pub pinned: bool,
    pub source_message_ids: Vec<String>,
}

/// An active memory and decoded normalized vector, used by Step 17 retrieval.
#[derive(Debug, Clone)]
pub struct MemoryCandidate {
    pub memory: Memory,
    pub embedding: Vec<f32>,
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
