// src/ai/caption.rs
// AI-powered streaming caption generator.
//
// Supports Anthropic (Claude Vision) and LM Studio (multimodal local models).
// Each caption field is emitted as a `caption-field` Tauri event as it arrives.

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

// ── Shared types ──────────────────────────────────────────────────────────────

/// One caption field arriving from the stream.
#[derive(Debug, Clone, Serialize)]
pub struct CaptionFieldEvent {
    pub field: String,
    pub value: serde_json::Value,
}

#[derive(Deserialize)]
struct NdjsonLine {
    field: String,
    value: serde_json::Value,
}

// ── System prompt (shared) ────────────────────────────────────────────────────

const SYSTEM: &str = r#"
You are a professional photo editor captioning street and documentary images for editorial submission.

Study the photograph carefully, then output EXACTLY FIVE lines of NDJSON (no markdown, no prose, no preamble):

{"field":"title","value":"..."}
{"field":"description","value":"..."}
{"field":"keywords","value":["...","...",...]}
{"field":"location","value":"..."}
{"field":"credit","value":""}

Field rules:
  title       — 5-10 words. Evocative, newspaper-style headline. Specific beats generic.
                Bad: "Man Walking in City"  Good: "Rush Hour Solitude, Financial District"
  description — 2-3 sentences. Journalistic present tense. Describe concretely what you see:
                subject, action, setting, mood, light. Never speculate about identities.
  keywords    — 8-15 lowercase terms. Mix specific (what's in frame) + thematic (genre/mood).
                Include: subject nouns, action verbs, setting, time of day, photographic style.
                Example: ["street photography","rain","reflection","umbrella","commuter","city","monochrome","motion blur"]
  location    — "City, Country" format if identifiable from scene clues. Empty string if unknown.
  credit      — Always output an empty string "". The photographer fills this in themselves.

IMPORTANT: Output ONLY the five NDJSON lines, nothing else.
"#;

// ── Shared flush helper ───────────────────────────────────────────────────────

fn flush_fields(buf: &mut String, caption: &mut crate::core::caption::Caption, handle: &AppHandle) {
    while let Some(nl) = buf.find('\n') {
        let line = buf[..nl].trim().to_string();
        *buf = buf[nl + 1..].to_string();
        if line.is_empty() {
            continue;
        }

        if let Ok(parsed) = serde_json::from_str::<NdjsonLine>(&line) {
            match parsed.field.as_str() {
                "title" => {
                    if let Some(s) = parsed.value.as_str() {
                        caption.title = s.to_string();
                    }
                }
                "description" => {
                    if let Some(s) = parsed.value.as_str() {
                        caption.description = s.to_string();
                    }
                }
                "keywords" => {
                    if let Some(arr) = parsed.value.as_array() {
                        caption.keywords = arr
                            .iter()
                            .filter_map(|v| v.as_str())
                            .map(str::to_string)
                            .collect();
                    }
                }
                "location" => {
                    if let Some(s) = parsed.value.as_str() {
                        caption.location = s.to_string();
                    }
                }
                "credit" => {
                    if let Some(s) = parsed.value.as_str() {
                        caption.credit = s.to_string();
                    }
                }
                _ => {}
            }
            let _ = handle.emit(
                "caption-field",
                CaptionFieldEvent {
                    field: parsed.field,
                    value: parsed.value,
                },
            );
        }
    }
}

// ── Anthropic implementation ──────────────────────────────────────────────────

#[cfg(feature = "provider-anthropic")]
mod anthropic_impl {
    use super::{flush_fields, SYSTEM};
    use anyhow::{bail, Result};
    use futures_util::StreamExt;
    use reqwest::Client;
    use serde::Deserialize;
    use serde_json::json;
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
        jpeg_b64: &str,
        handle: &AppHandle,
    ) -> Result<crate::core::caption::Caption> {
        let body = json!({
            "model":      MODEL,
            "max_tokens": 600,
            "stream":     true,
            "system":     SYSTEM,
            "messages": [{
                "role": "user",
                "content": [
                    {
                        "type": "image",
                        "source": {
                            "type":       "base64",
                            "media_type": "image/jpeg",
                            "data":       jpeg_b64,
                        }
                    },
                    { "type": "text", "text": "Caption this photograph." }
                ]
            }]
        });

        let client = Client::builder().timeout(Duration::from_secs(60)).build()?;
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
        let mut caption = crate::core::caption::Caption::default();

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
                                        flush_fields(&mut text_buf, &mut caption, handle);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        text_buf.push('\n');
        flush_fields(&mut text_buf, &mut caption, handle);
        Ok(caption)
    }
}

#[cfg(feature = "provider-anthropic")]
pub use anthropic_impl::run;

// ── LM Studio implementation ──────────────────────────────────────────────────

// Single JSON-object prompt — far more reliable with local models than NDJSON.
// `response_format: json_object` in the request body forces JSON-mode output.
const SYSTEM_LMSTUDIO: &str = r#"
You are a professional photo editor captioning street and documentary photographs.

Return ONLY a single valid JSON object — no markdown, no code fences, no explanation.
The JSON object must have exactly these keys:

{"title":"...","description":"...","keywords":["..."],"location":"...","credit":""}

Rules:
  title       — 5-10 words, newspaper-style headline
  description — 2-3 sentences, journalistic present tense
  keywords    — array of 8-15 lowercase terms
  location    — "City, Country" if identifiable, otherwise ""
  credit      — always output ""
"#;

pub async fn run_lmstudio(
    base_url: &str,
    model: &str,
    jpeg_b64: &str,
    handle: &AppHandle,
) -> anyhow::Result<crate::core::caption::Caption> {
    use crate::ai::lmstudio;
    use anyhow::bail;

    // 4096 tokens: reasoning models (e.g. Gemma 4) spend several hundred tokens
    // on internal chain-of-thought before outputting the answer; the content JSON
    // itself is only ~200 tokens, but we need headroom for the thinking phase.
    let body = lmstudio::build_vision_body(
        model,
        SYSTEM_LMSTUDIO,
        jpeg_b64,
        "Caption this photograph.",
        4096,
    );

    let client = lmstudio::new_client()?;
    let response = client
        .post(lmstudio::chat_url(base_url))
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        bail!("LM Studio error {status}: {text}");
    }

    // Collect the full response body, then parse SSE events line-by-line.
    // Using response.text() + .lines() handles \n / \r\n / \r uniformly and
    // avoids the manual byte-buffer scan that silently drops chunks without
    // trailing newlines.
    let body = response.text().await?;
    tracing::info!("LM Studio caption raw response:\n{}", body);

    // Non-streaming response: choices[0].message.content holds the full answer.
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

    tracing::info!("LM Studio caption content:\n{}", full_text);

    let mut caption = crate::core::caption::Caption::default();

    // Find the first complete JSON object in the output and apply it.
    // This handles markdown fences, preamble text, etc.
    'parse: for obj in lmstudio::extract_json_objects(&full_text) {
        let Ok(val) = serde_json::from_str::<serde_json::Value>(&obj) else {
            continue;
        };

        let fields = [
            ("title", val.get("title")),
            ("description", val.get("description")),
            ("keywords", val.get("keywords")),
            ("location", val.get("location")),
            ("credit", val.get("credit")),
        ];

        // Only accept an object that has at least title or description.
        if fields[0].1.is_none() && fields[1].1.is_none() {
            continue;
        }

        for (key, maybe_val) in &fields {
            let Some(v) = maybe_val else { continue };
            match *key {
                "title" => {
                    if let Some(s) = v.as_str() {
                        caption.title = s.to_string();
                    }
                }
                "description" => {
                    if let Some(s) = v.as_str() {
                        caption.description = s.to_string();
                    }
                }
                "keywords" => {
                    if let Some(arr) = v.as_array() {
                        caption.keywords = arr
                            .iter()
                            .filter_map(|x| x.as_str())
                            .map(str::to_string)
                            .collect();
                    }
                }
                "location" => {
                    if let Some(s) = v.as_str() {
                        caption.location = s.to_string();
                    }
                }
                "credit" => {
                    if let Some(s) = v.as_str() {
                        caption.credit = s.to_string();
                    }
                }
                _ => {}
            }
            let _ = handle.emit(
                "caption-field",
                CaptionFieldEvent {
                    field: key.to_string(),
                    value: (*v).clone(),
                },
            );
        }
        break 'parse;
    }

    // Fallback: extract individual string fields from partial/truncated JSON.
    // Reasoning models sometimes hit max_tokens mid-JSON; we salvage completed fields.
    if caption.title.is_empty() {
        tracing::warn!("LM Studio caption: complete JSON not found — trying partial extraction");

        let emit_str = |field: &str, val: String| {
            let v = serde_json::Value::String(val.clone());
            let _ = handle.emit(
                "caption-field",
                CaptionFieldEvent {
                    field: field.to_string(),
                    value: v,
                },
            );
            val
        };

        if let Some(s) = lmstudio::extract_str_field(&full_text, "title") {
            caption.title = emit_str("title", s);
        }
        if let Some(s) = lmstudio::extract_str_field(&full_text, "description") {
            caption.description = emit_str("description", s);
        }
        if let Some(arr) = lmstudio::extract_str_array_field(&full_text, "keywords") {
            caption.keywords = arr.clone();
            let _ = handle.emit(
                "caption-field",
                CaptionFieldEvent {
                    field: "keywords".into(),
                    value: serde_json::Value::Array(
                        arr.into_iter().map(serde_json::Value::String).collect(),
                    ),
                },
            );
        }
        if let Some(s) = lmstudio::extract_str_field(&full_text, "location") {
            caption.location = emit_str("location", s);
        }
    }

    Ok(caption)
}
