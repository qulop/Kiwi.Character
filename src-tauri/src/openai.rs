//! Thin OpenAI-compatible HTTP client.
//!
//! Works against any server that speaks the OpenAI API surface — LM Studio,
//! Ollama (`/v1`), llama.cpp server, etc. We hand-roll the calls we need rather
//! than pulling a full SDK so quirks of local servers (no auth, odd base URLs)
//! stay easy to handle.

use std::time::Duration;

use futures_util::StreamExt;
use serde::{Deserialize, Serialize};

use crate::models::{ModelLoadResult, ModelSettings};

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

    return Ok(parsed.data.into_iter().map(|m| m.id).collect());
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

    return Ok(parsed
        .data
        .into_iter()
        .filter(|m| m.state == "loaded" && (m.kind == "llm" || m.kind == "vlm"))
        .map(|m| m.id)
        .collect());
}

/// Ask LM Studio's native API to load `settings.model` with the requested
/// context window. This cannot use the OpenAI-compatible chat endpoint: that
/// endpoint only triggers JIT loading with LM Studio's default load settings.
pub async fn load_model(settings: &ModelSettings) -> Result<ModelLoadResult, String> {
    let url = lmstudio_load_url(&settings.endpoint);
    let context_length = context_length_tokens(settings.context_length)?;
    let body = LmStudioLoadReq {
        model: &settings.model,
        context_length,
        // Return the server's actual applied configuration so the UI can report
        // it instead of assuming the requested value was accepted.
        echo_load_config: true,
    };

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
        .map_err(|e| format!("Could not reach LM Studio's native API at {url}: {e}"))?;

    let status = resp.status();
    if !status.is_success() {
        let detail = resp.text().await.unwrap_or_default();
        return Err(format!("LM Studio model load failed (HTTP {status}): {detail}"));
    }

    let parsed: LmStudioLoadResp = resp
        .json()
        .await
        .map_err(|e| format!("Unexpected LM Studio model-load response: {e}"))?;
    return Ok(ModelLoadResult {
        context_length: parsed.load_config.and_then(|config| config.context_length),
    });
}

/// The app endpoint is normally `http://host:port/v1`; LM Studio's native
/// model-management endpoints live at the host root under `/api/v1`.
fn lmstudio_load_url(endpoint: &str) -> String {
    let base = endpoint.trim_end_matches('/');
    let base = base.strip_suffix("/v1").unwrap_or(base).trim_end_matches('/');
    return format!("{base}/api/v1/models/load");
}

/// The UI stores context length in thousands of tokens (e.g. `100` = `100k`).
/// A zero value preserves the old "server default" behavior by omitting the
/// optional native-API field.
fn context_length_tokens(context_length_k: i64) -> Result<Option<i64>, String> {
    if context_length_k < 0 {
        return Err("Context length cannot be negative.".into());
    }
    if context_length_k == 0 {
        return Ok(None);
    }
    return context_length_k
        .checked_mul(1_000)
        .map(Some)
        .ok_or_else(|| "Context length is too large.".into());
}

#[derive(Serialize)]
struct LmStudioLoadReq<'a> {
    model: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    context_length: Option<i64>,
    echo_load_config: bool,
}

#[derive(Deserialize)]
struct LmStudioLoadResp {
    #[serde(default)]
    load_config: Option<LmStudioLoadConfig>,
}

#[derive(Deserialize)]
struct LmStudioLoadConfig {
    #[serde(default)]
    context_length: Option<i64>,
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
        return ChatReq {
            model: &settings.model,
            messages,
            temperature: settings.temperature,
            max_tokens: (settings.max_tokens > 0).then_some(settings.max_tokens),
            stream,
        };
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
        #[serde(default)]
        content: String,
        // Reasoning models (e.g. Qwen3) put the "thinking" here.
        #[serde(default)]
        reasoning_content: String,
    }

    let parsed: ChatResp = resp
        .json()
        .await
        .map_err(|e| format!("Unexpected chat response: {e}"))?;

    let msg = parsed
        .choices
        .into_iter()
        .next()
        .map(|c| c.message)
        .ok_or_else(|| "LLM returned no choices".to_string())?;

    // The answer is in `content`; if the model only produced reasoning, fall
    // back to it so the reply isn't blank.
    return Ok(if msg.content.trim().is_empty() {
        msg.reasoning_content
    } else {
        msg.content
    });
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
    // only process complete lines. Track the answer (`content`) and the
    // reasoning-model "thinking" (`reasoning_content`) separately.
    let mut buf = String::new();
    let mut content_full = String::new();
    let mut reasoning_full = String::new();
    let mut stream = resp.bytes_stream();

    'outer: while let Some(chunk) = stream.next().await {
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
                break 'outer;
            }
            if data.is_empty() {
                continue;
            }

            // Tolerate frames that carry no content delta (role headers, etc.).
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(data) {
                let delta = &v["choices"][0]["delta"];
                if let Some(tok) = delta["content"].as_str() {
                    if !tok.is_empty() {
                        content_full.push_str(tok);
                        on_token(tok);
                    }
                }
                if let Some(rtok) = delta["reasoning_content"].as_str() {
                    reasoning_full.push_str(rtok);
                }
            }
        }
    }

    // If the model only produced "thinking" (no content), show the reasoning so
    // the reply isn't blank.
    if content_full.trim().is_empty() && !reasoning_full.trim().is_empty() {
        on_token(&reasoning_full);
        return Ok(reasoning_full);
    }
    return Ok(content_full);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_load_url_removes_the_openai_v1_suffix() {
        assert_eq!(
            lmstudio_load_url("http://localhost:1234/v1"),
            "http://localhost:1234/api/v1/models/load"
        );
        assert_eq!(
            lmstudio_load_url("http://localhost:1234/v1/"),
            "http://localhost:1234/api/v1/models/load"
        );
    }

    #[test]
    fn context_slider_value_becomes_tokens() {
        assert_eq!(context_length_tokens(100).unwrap(), Some(100_000));
        assert_eq!(context_length_tokens(4).unwrap(), Some(4_000));
        assert_eq!(context_length_tokens(0).unwrap(), None);
    }

    #[test]
    fn negative_context_is_rejected() {
        assert!(context_length_tokens(-1).is_err());
    }
}
