//! Thin OpenAI-compatible HTTP client.
//!
//! Works against any server that speaks the OpenAI API surface — LM Studio,
//! Ollama (`/v1`), llama.cpp server, etc. We hand-roll the calls we need rather
//! than pulling a full SDK so quirks of local servers (no auth, odd base URLs)
//! stay easy to handle.

use std::time::Duration;

use futures_util::StreamExt;
use serde::{Deserialize, Serialize};

use crate::models::ModelSettings;

/// `GET {endpoint}/models` — returns the ids the server advertises.
pub async fn list_models(endpoint: &str) -> Result<Vec<String>, String> {
    let url = format!("{}/models", endpoint.trim_end_matches('/'));

    // Short timeout so a bad/offline endpoint fails fast (used by Test + the
    // background health ping).
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Could not reach {url}: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("Endpoint returned HTTP {}", resp.status()));
    }

    #[derive(Deserialize)]
    struct ModelsResp {
        data: Vec<ModelEntry>,
    }
    #[derive(Deserialize)]
    struct ModelEntry {
        id: String,
    }

    let parsed: ModelsResp = resp
        .json()
        .await
        .map_err(|e| format!("Unexpected /models response: {e}"))?;

    Ok(parsed.data.into_iter().map(|m| m.id).collect())
}

/// The models currently **loaded** on the server, via LM Studio's native
/// `/api/v0/models` (which reports each model's `state`). Filters to LLM/VLM
/// models that are `loaded` (excludes embeddings and unloaded ones). Returns an
/// error for servers that don't expose `/api/v0` (e.g. plain Ollama).
pub async fn loaded_models(endpoint: &str) -> Result<Vec<String>, String> {
    // The native API lives at the host root, not under /v1.
    let base = endpoint.trim_end_matches('/');
    let base = base.strip_suffix("/v1").unwrap_or(base).trim_end_matches('/');
    let url = format!("{base}/api/v0/models");

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Could not reach {url}: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Endpoint returned HTTP {}", resp.status()));
    }

    #[derive(Deserialize)]
    struct V0Resp {
        data: Vec<V0Model>,
    }
    #[derive(Deserialize)]
    struct V0Model {
        id: String,
        #[serde(default)]
        state: String,
        #[serde(default, rename = "type")]
        kind: String,
    }

    let parsed: V0Resp = resp
        .json()
        .await
        .map_err(|e| format!("Unexpected /api/v0/models response: {e}"))?;

    Ok(parsed
        .data
        .into_iter()
        .filter(|m| m.state == "loaded" && (m.kind == "llm" || m.kind == "vlm"))
        .map(|m| m.id)
        .collect())
}

/// Ask the server to load `settings.model`. There is no standard OpenAI "load"
/// endpoint, but LM Studio (with Just-In-Time loading, on by default) loads the
/// requested model when it receives a chat request for it. We send a minimal
/// 1-token request; when it returns successfully the model is loaded/resident.
pub async fn load_model(settings: &ModelSettings) -> Result<(), String> {
    let url = format!("{}/chat/completions", settings.endpoint.trim_end_matches('/'));
    let body = serde_json::json!({
        "model": settings.model,
        "messages": [{ "role": "user", "content": "Hi" }],
        "max_tokens": 1,
        "stream": false,
    });

    // Loading a large model can take a while — allow a generous timeout.
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(900))
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Could not reach {url}: {e}"))?;

    let status = resp.status();
    if !status.is_success() {
        let detail = resp.text().await.unwrap_or_default();
        return Err(format!("Model load failed (HTTP {status}): {detail}"));
    }
    Ok(())
}

#[derive(Serialize)]
pub struct ChatReqMsg {
    pub role: String,
    pub content: String,
}

/// The request body we send to `/chat/completions`.
///
/// `max_tokens` is omitted when non-positive so we never over-request tokens
/// relative to the model's context (a common cause of LM Studio "Channel
/// Error" mid-generation).
#[derive(Serialize)]
struct ChatReq<'a> {
    model: &'a str,
    messages: Vec<ChatReqMsg>,
    temperature: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<i64>,
    stream: bool,
}

impl<'a> ChatReq<'a> {
    fn build(settings: &'a ModelSettings, messages: Vec<ChatReqMsg>, stream: bool) -> Self {
        ChatReq {
            model: &settings.model,
            messages,
            temperature: settings.temperature,
            max_tokens: (settings.max_tokens > 0).then_some(settings.max_tokens),
            stream,
        }
    }
}

/// `POST {endpoint}/chat/completions` — non-streaming. Returns the assistant text.
pub async fn chat_completion(
    settings: &ModelSettings,
    messages: Vec<ChatReqMsg>,
) -> Result<String, String> {
    let url = format!("{}/chat/completions", settings.endpoint.trim_end_matches('/'));
    let body = ChatReq::build(settings, messages, false);

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(300))
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Could not reach {url}: {e}"))?;

    let status = resp.status();
    if !status.is_success() {
        // Surface the server's own message — this is what tells the user *why*
        // the model refused (bad request field, context overflow, etc.).
        let detail = resp.text().await.unwrap_or_default();
        return Err(format!("LLM endpoint returned HTTP {status}: {detail}"));
    }

    #[derive(Deserialize)]
    struct ChatResp {
        choices: Vec<Choice>,
    }
    #[derive(Deserialize)]
    struct Choice {
        message: RespMsg,
    }
    #[derive(Deserialize)]
    struct RespMsg {
        content: String,
    }

    let parsed: ChatResp = resp
        .json()
        .await
        .map_err(|e| format!("Unexpected chat response: {e}"))?;

    parsed
        .choices
        .into_iter()
        .next()
        .map(|c| c.message.content)
        .ok_or_else(|| "LLM returned no choices".to_string())
}

/// `POST {endpoint}/chat/completions` with `stream: true`.
///
/// Parses the OpenAI Server-Sent-Events stream, invoking `on_token` for each
/// content delta, and returns the full concatenated text once the server sends
/// `data: [DONE]` (or the stream ends).
pub async fn chat_completion_stream<F: FnMut(&str)>(
    settings: &ModelSettings,
    messages: Vec<ChatReqMsg>,
    mut on_token: F,
) -> Result<String, String> {
    let url = format!("{}/chat/completions", settings.endpoint.trim_end_matches('/'));
    let body = ChatReq::build(settings, messages, true);

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(600))
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Could not reach {url}: {e}"))?;

    let status = resp.status();
    if !status.is_success() {
        let detail = resp.text().await.unwrap_or_default();
        return Err(format!("LLM endpoint returned HTTP {status}: {detail}"));
    }

    // SSE frames may split across network chunks — accumulate in a buffer and
    // only process complete lines.
    let mut buf = String::new();
    let mut full = String::new();
    let mut stream = resp.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let bytes = chunk.map_err(|e| format!("Stream error: {e}"))?;
        buf.push_str(&String::from_utf8_lossy(&bytes));

        while let Some(nl) = buf.find('\n') {
            let line: String = buf.drain(..=nl).collect();
            let line = line.trim();

            let Some(data) = line.strip_prefix("data:") else {
                continue;
            };
            let data = data.trim();
            if data == "[DONE]" {
                return Ok(full);
            }
            if data.is_empty() {
                continue;
            }

            // Tolerate frames that carry no content delta (role headers, etc.).
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(data) {
                if let Some(tok) = v["choices"][0]["delta"]["content"].as_str() {
                    full.push_str(tok);
                    on_token(tok);
                }
            }
        }
    }

    Ok(full)
}
