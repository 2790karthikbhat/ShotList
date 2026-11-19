// src/ai/sequence.rs
// AI-powered zine narrative sequencer.
//
// Supports Anthropic (Claude) and LM Studio (OpenAI-compatible local models).
// Reads captions and proposes the best narrative page ordering.
//
// Output is NDJSON:
//   {"position": 0, "image_id": "<uuid>", "reason": "..."}  (one per page)
//   {"summary": "..."}  (final overview line)
//
// Each line emits a `sequence-suggestion` or `sequence-summary` Tauri event.

use serde::{Deserialize, Serialize};

#[cfg(feature = "provider-anthropic")]
pub use inner::run;
#[cfg(not(feature = "provider-anthropic"))]
pub use stub::run;

// ── Shared event type ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SequenceSuggestion {
    pub position: usize,
    pub image_id: String,
    pub reason: String,
}

// ── System prompt (shared by both providers) ──────────────────────────────────

const SYSTEM: &str = r#"
You are a photo editor specialising in documentary and street photography zines.
Given a list of images with their captions in their current order, propose a new
narrative sequence that creates emotional flow: a strong opening, building tension,
a crescendo moment, and resolution.

Output NDJSON — one JSON object per line, in the new desired order:
{"position": 0, "image_id": "<uuid>", "reason": "<max 20 words explaining placement>"}

After all page lines, output exactly one summary line:
{"summary": "<2-3 sentences describing the narrative arc>"}

Rules:
• Use every image exactly once.
• Output only valid JSON lines — no markdown, no prose outside JSON.
• Positions must be 0-indexed and contiguous.
"#;

// ── Shared flush helper ───────────────────────────────────────────────────────

fn flush_suggestions(buf: &mut String, handle: &tauri::AppHandle, result: &mut [String]) {
    use tauri::Emitter;
    while let Some(nl) = buf.find('\n') {
        let line = buf[..nl].trim().to_string();
        *buf = buf[nl + 1..].to_string();
        if line.is_empty() {
            continue;
        }

        let Ok(val) = serde_json::from_str::<serde_json::Value>(&line) else {
            continue;
        };

        if let Some(summary) = val.get("summary").and_then(|v| v.as_str()) {
            let _ = handle.emit("sequence-summary", summary.to_string());
            continue;
        }

        if let (Some(pos), Some(image_id), Some(reason)) = (
            val.get("position").and_then(|v| v.as_u64()),
            val.get("image_id").and_then(|v| v.as_str()),
            val.get("reason").and_then(|v| v.as_str()),
        ) {
            let suggestion = SequenceSuggestion {
                position: pos as usize,
                image_id: image_id.to_string(),
                reason: reason.to_string(),
            };
            let _ = handle.emit("sequence-suggestion", &suggestion);
            if (pos as usize) < result.len() {
                result[pos as usize] = image_id.to_string();
            }
        }
    }
}

// ── Anthropic implementation ──────────────────────────────────────────────────

#[cfg(feature = "provider-anthropic")]
mod inner {
    use super::{flush_suggestions, SYSTEM};
    use crate::core::caption::Caption;
    use crate::core::zine::ZinePage;
    use anyhow::{bail, Result};
    use futures_util::StreamExt;
    use reqwest::Client;
    use serde::Deserialize;
    use serde_json::json;
    use std::collections::HashMap;
    use std::time::Duration;
    use tauri::AppHandle;

    const API_URL: &str = "https://api.anthropic.com/v1/messages";
    const MODEL: &str = "claude-opus-4-5";
    const VERSION: &str = "2023-06-01";

    #[derive(Deserialize)]
    struct SseDelta {
        #[serde(rename = "type")]
        kind: String,
        delta: Option<TextDelta>,
    }
    #[derive(Deserialize)]
    struct TextDelta {
        #[serde(rename = "type")]
        kind: String,
        text: Option<String>,
    }

    pub async fn run(
        api_key: &str,
        pages: &[ZinePage],
        captions: &HashMap<String, Caption>,
        handle: &AppHandle,
    ) -> Result<Vec<String>> {
        let mut lines = Vec::new();
        for (i, page) in pages.iter().enumerate() {
            let cap_text = page
                .image_id
                .as_ref()
                .and_then(|id| captions.get(id))
                .map(|c| {
                    let mut parts = Vec::new();
                    if !c.title.is_empty() {
                        parts.push(format!("title: {}", c.title));
                    }
                    if !c.description.is_empty() {
                        parts.push(format!("desc: {}", c.description));
                    }
                    if !c.location.is_empty() {
                        parts.push(format!("loc: {}", c.location));
                    }
                    parts.join("; ")
                })
                .unwrap_or_else(|| "(no caption)".to_string());
            let image_id = page.image_id.as_deref().unwrap_or("none");
            lines.push(format!("{i}. image_id={image_id} — {cap_text}"));
        }

        let user_prompt = format!(
            "Current page order (index. image_id — caption):\n\n{}\n\n\
             Propose the best narrative sequence using every image exactly once.",
            lines.join("\n")
        );

        let body = json!({
            "model":      MODEL,
            "max_tokens": 1024,
            "stream":     true,
            "system":     SYSTEM,
            "messages": [{ "role": "user", "content": user_prompt }]
        });

        let client = Client::builder().timeout(Duration::from_secs(90)).build()?;
        let response = client
            .post(API_URL)
            .header("x-api-key", api_key)
            .header("anthropic-version", VERSION)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            bail!("Anthropic API error {status}: {text}");
        }

        let mut stream = response.bytes_stream();
        let mut raw_buf = String::new();
        let mut text_buf = String::new();
        let mut result: Vec<String> = vec![String::new(); pages.len()];

        while let Some(chunk) = stream.next().await {
            let bytes = chunk?;
            raw_buf.push_str(&String::from_utf8_lossy(&bytes));
            while let Some(nl) = raw_buf.find('\n') {
                let line = raw_buf[..nl].trim().to_string();
                raw_buf = raw_buf[nl + 1..].to_string();
                if let Some(data) = line.strip_prefix("data: ") {
                    if data == "[DONE]" {
                        break;
                    }
                    if let Ok(evt) = serde_json::from_str::<SseDelta>(data) {
                        if evt.kind == "content_block_delta" {
                            if let Some(delta) = evt.delta {
                                if delta.kind == "text_delta" {
                                    if let Some(text) = delta.text {
                                        text_buf.push_str(&text);
                                        flush_suggestions(&mut text_buf, handle, &mut result);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        text_buf.push('\n');
        flush_suggestions(&mut text_buf, handle, &mut result);
        Ok(result)
    }
}

// ── Stub (no AI provider) ─────────────────────────────────────────────────────

#[cfg(not(feature = "provider-anthropic"))]
mod stub {
    use crate::core::caption::Caption;
    use crate::core::zine::ZinePage;
    use anyhow::Result;
    use std::collections::HashMap;
    use tauri::AppHandle;

    pub async fn run(
        _api_key: &str,
        pages: &[ZinePage],
        _captions: &HashMap<String, Caption>,
        _handle: &AppHandle,
    ) -> Result<Vec<String>> {
        Ok(pages.iter().filter_map(|p| p.image_id.clone()).collect())
    }
}

// ── LM Studio system prompt ───────────────────────────────────────────────────

// Single JSON-object format — more reliable with local models than NDJSON.
const SYSTEM_LMSTUDIO: &str = r#"
You are a photo editor specialising in documentary and street photography zines.
Given a list of images with their captions in their current order, propose a new
narrative sequence that creates emotional flow: a strong opening, building tension,
a crescendo moment, and resolution.

Return ONLY a single valid JSON object — no markdown, no code fences, no explanation.
The JSON object must have exactly this structure:

{
  "sequence": [
    {"position": 0, "image_id": "<uuid>", "reason": "<max 20 words>"},
    ...
  ],
  "summary": "<2-3 sentences describing the narrative arc>"
}

Rules:
• Use every image exactly once.
• Positions must be 0-indexed and contiguous.
"#;

// ── LM Studio implementation ──────────────────────────────────────────────────

pub async fn run_lmstudio(
    base_url: &str,
    model: &str,
    pages: &[crate::core::zine::ZinePage],
    captions: &std::collections::HashMap<String, crate::core::caption::Caption>,
    handle: &tauri::AppHandle,
) -> anyhow::Result<Vec<String>> {
    use crate::ai::lmstudio;
    use anyhow::bail;
    use tauri::Emitter;

    let mut lines = Vec::new();
    for (i, page) in pages.iter().enumerate() {
        let cap_text = page
            .image_id
            .as_ref()
            .and_then(|id| captions.get(id))
            .map(|c| {
                let mut parts = Vec::new();
                if !c.title.is_empty() {
                    parts.push(format!("title: {}", c.title));
                }
                if !c.description.is_empty() {
                    parts.push(format!("desc: {}", c.description));
                }
                if !c.location.is_empty() {
                    parts.push(format!("loc: {}", c.location));
                }
                parts.join("; ")
            })
            .unwrap_or_else(|| "(no caption)".to_string());
        let image_id = page.image_id.as_deref().unwrap_or("none");
        lines.push(format!("{i}. image_id={image_id} — {cap_text}"));
    }

    let user_prompt = format!(
        "Current page order (index. image_id — caption):\n\n{}\n\n\
         Propose the best narrative sequence using every image exactly once.",
        lines.join("\n")
    );

    let body_req = lmstudio::build_text_body(model, SYSTEM_LMSTUDIO, &user_prompt, 4096);

    let client = lmstudio::new_client()?;
    let response = client
        .post(lmstudio::chat_url(base_url))
        .header("content-type", "application/json")
        .json(&body_req)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        bail!("LM Studio error {status}: {text}");
    }

    let body = response.text().await?;
    tracing::info!("LM Studio sequence raw response:\n{}", body);

    let full_text = serde_json::from_str::<serde_json::Value>(&body)
        .ok()
        .and_then(|v| {
            v.get("choices")?
                .as_array()?
                .first()
                .and_then(|c| c.get("message"))
                .and_then(|m| m.get("content"))
                .and_then(|c| c.as_str())
                .map(str::to_string)
        })
        .unwrap_or_default();

    tracing::info!("LM Studio sequence content:\n{}", full_text);

    let mut result: Vec<String> = vec![String::new(); pages.len()];

    'parse: for obj in lmstudio::extract_json_objects(&full_text) {
        let Ok(val) = serde_json::from_str::<serde_json::Value>(&obj) else {
            continue;
        };
        let Some(arr) = val.get("sequence").and_then(|v| v.as_array()) else {
            continue;
        };

        for item in arr {
            if let (Some(pos), Some(image_id), Some(reason)) = (
                item.get("position").and_then(|v| v.as_u64()),
                item.get("image_id").and_then(|v| v.as_str()),
                item.get("reason").and_then(|v| v.as_str()),
            ) {
                let suggestion = SequenceSuggestion {
                    position: pos as usize,
                    image_id: image_id.to_string(),
                    reason: reason.to_string(),
                };
                let _ = handle.emit("sequence-suggestion", &suggestion);
                if (pos as usize) < result.len() {
                    result[pos as usize] = image_id.to_string();
                }
            }
        }

        if let Some(summary) = val.get("summary").and_then(|v| v.as_str()) {
            let _ = handle.emit("sequence-summary", summary.to_string());
        }

        break 'parse;
    }

    Ok(result)
}
