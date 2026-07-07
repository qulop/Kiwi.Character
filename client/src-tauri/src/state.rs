//! In-memory application state.
//!
//! Everything lives behind a single `Mutex` for this first pass — no disk
//! persistence yet, so data resets on restart. Swap `Store` for a JSON/SQLite
//! backing later without touching the command signatures.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::models::{Character, ChatMessage, ModelSettings};

/// A single conversation thread with one character.
pub struct Conversation {
    pub id: String,
    pub character_id: String,
    pub messages: Vec<ChatMessage>,
}

#[derive(Default)]
pub struct Store {
    pub characters: Vec<Character>,
    pub conversations: HashMap<String, Conversation>,
    pub settings: ModelSettings,
}

pub struct AppState {
    pub store: Mutex<Store>,
}

impl AppState {
    /// State seeded with a few sample characters + one prior conversation so
    /// the UI is fully clickable on first launch.
    pub fn seeded() -> Self {
        let characters = vec![
            Character {
                id: "aria".into(),
                name: "Aria".into(),
                info: "Curious AI companion who loves to learn".into(),
                avatar: None,
                appearance: "A warm, holographic presence with shifting violet hues.".into(),
                description: "Aria is endlessly curious, upbeat, and a little playful. \
                    She asks thoughtful follow-up questions and enjoys a good tangent."
                    .into(),
                initial_message: "Hi there! I'm Aria. What's on your mind today?".into(),
            },
            Character {
                id: "sherlock".into(),
                name: "Sherlock Holmes".into(),
                info: "Brilliant, observant consulting detective".into(),
                avatar: None,
                appearance: "Tall and lean, sharp features, often in a long coat.".into(),
                description: "The world's only consulting detective. Deductive, blunt, \
                    impatient with sloppy thinking, but loyal to those he respects."
                    .into(),
                initial_message:
                    "Ah, a visitor. Sit. You've clearly come about a problem — out with it."
                        .into(),
            },
            Character {
                id: "luna".into(),
                name: "Luna".into(),
                info: "Dreamy poet with a love for the night sky".into(),
                avatar: None,
                appearance: "Soft silver hair, star-flecked eyes, always a little distant.".into(),
                description: "Luna speaks in gentle, lyrical language and finds wonder in \
                    small things. Calm, kind, and quietly wise."
                    .into(),
                initial_message: "Oh, hello… I was just watching the stars. Care to join me?"
                    .into(),
            },
        ];

        // Seed one prior conversation so the History list isn't empty.
        let mut conversations = HashMap::new();
        let conv_id = "conv-sherlock".to_string();
        conversations.insert(
            conv_id.clone(),
            Conversation {
                id: conv_id,
                character_id: "sherlock".into(),
                messages: vec![ChatMessage {
                    id: new_id(),
                    role: "assistant".into(),
                    content:
                        "Ah, a visitor. Sit. You've clearly come about a problem — out with it."
                            .into(),
                    created_at: Some(now_ms()),
                }],
            },
        );

        Self {
            store: Mutex::new(Store {
                characters,
                conversations,
                settings: ModelSettings::default(),
            }),
        }
    }
}

/// Current time in milliseconds since the Unix epoch.
pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// A fresh random id.
pub fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}
