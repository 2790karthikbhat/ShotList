// src/ai/lmstudio.rs
// OpenAI-compatible client helpers for LM Studio.
//
// LM Studio exposes an OpenAI-compatible API (default http://localhost:1234/v1).
// SSE chunk format differs from Anthropic:
//   Anthropic: {"type":"content_block_delta","delta":{"type":"text_delta","text":"..."}}
//   LM Studio: {"choices":[{"delta":{"content":"..."}}]}

use anyhow::{bail, Result};
use reqwest::Client;
use serde::Deserialize;
use serde_json::{json, Value};
use std::time::Duration;

// ── Request body builders ─────────────────────────────────────────────────────

/// Text-only chat completion body (for sequence suggestions).
/// Non-streaming — simpler to parse and avoids SSE field variations across models.
pub fn build_text_body(model: &str, system: &str, user_prompt: &str, max_tokens: u32) -> Value {
    json!({
        "model": model,
        "max_tokens": max_tokens,
        "stream": false,
        "messages": [
            { "role": "system", "content": system },
            { "role": "user",   "content": user_prompt },
        ]
    })
}

/// Vision chat completion body (image + text, for develop & caption).
///
/// Non-streaming — avoids SSE parsing entirely. Reasoning models (e.g. Gemma 4)
/// use `reasoning_content` in delta chunks rather than `content`, which would
/// silently discard the actual answer in streaming mode.
///
/// `"detail": "low"` — processes the image at 512×512 internally (~85 tokens)
/// instead of tile-processing the full resolution (~1000+ tokens), reducing
/// KV-cache pressure on local hardware.
pub fn build_vision_body(
    model: &str,
    system: &str,
    jpeg_b64: &str,
    text_prompt: &str,
    max_tokens: u32,
) -> Value {
    json!({
        "model": model,
        "max_tokens": max_tokens,
        "stream": false,
        "messages": [
            { "role": "system", "content": system },
            {
                "role": "user",
                "content": [
                    {
                        "type": "image_url",
                        "image_url": {
                            "url":    format!("data:image/jpeg;base64,{}", jpeg_b64),
                            "detail": "low"
                        }
                    },
                    { "type": "text", "text": text_prompt }
                ]
            }
        ]
    })
}

// ── HTTP helpers ──────────────────────────────────────────────────────────────

pub fn chat_url(base_url: &str) -> String {
    format!("{}/chat/completions", base_url.trim_end_matches('/'))
}

pub fn new_client() -> Result<Client> {
    Ok(Client::builder()
        .timeout(Duration::from_secs(120))
        .build()?)
}

// ── Robust JSON extraction ────────────────────────────────────────────────────

/// Scan arbitrary text for complete, top-level JSON objects.
///
/// Models often wrap their output in markdown code fences or add a prose
/// preamble ("Here are the captions:"). This function finds every `{...}`
/// regardless of surrounding text, handling nested braces and quoted strings.
pub fn extract_json_objects(text: &str) -> Vec<String> {
    let mut results = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] != '{' {
            i += 1;
            continue;
        }

        let start = i;
        let mut depth = 0usize;
        let mut in_str = false;
        let mut escape = false;

        while i < chars.len() {
            let c = chars[i];
            if escape {
                escape = false;
            } else if in_str {
                match c {
                    '\\' => escape = true,
                    '"' => in_str = false,
                    _ => {}
                }
            } else {
                match c {
                    '"' => in_str = true,
                    '{' => depth += 1,
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            results.push(chars[start..=i].iter().collect::<String>());
                            break;
                        }
                    }
                    _ => {}
                }
            }
            i += 1;
        }

        if depth > 0 {
            break;
        } // incomplete object — wait for more data
        i += 1;
    }

    results
}

// ── Partial JSON field extractors ─────────────────────────────────────────────

/// Extract a single string field from potentially incomplete JSON.
///
/// Used when a reasoning model hits max_tokens mid-response and the outer
/// `extract_json_objects` scanner can't find a complete `{...}` object.
/// Returns `None` if the field is missing or its value hasn't started yet.
/// Returns a partial string if the closing `"` is missing (token cutoff).
pub fn extract_str_field(text: &str, field: &str) -> Option<String> {
    let needle = format!("\"{}\"", field);
    let field_pos = text.find(&needle)?;
    let after_key = text[field_pos + needle.len()..].trim_start();
    let colon_rest = after_key.strip_prefix(':')?.trim_start();

    if !colon_rest.starts_with('"') {
        return None;
    }

    let mut result = String::new();
    let mut escape = false;
    for c in colon_rest[1..].chars() {
        if escape {
            result.push(c);
            escape = false;
        } else if c == '\\' {
            escape = true;
        } else if c == '"' {
            return Some(result); // complete string
        } else {
            result.push(c);
        }
    }

    // Token cutoff mid-string — return what completed.
    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}

/// Extract an array-of-strings field from potentially incomplete JSON.
/// Only returns `Some` when the closing `]` is present (no partial arrays).
pub fn extract_str_array_field(text: &str, field: &str) -> Option<Vec<String>> {
    let needle = format!("\"{}\"", field);
    let field_pos = text.find(&needle)?;
    let after_key = text[field_pos + needle.len()..].trim_start();
    let colon_rest = after_key.strip_prefix(':')?.trim_start();

    if !colon_rest.starts_with('[') {
        return None;
    }

    // Walk forward to find the matching `]`.
    let mut depth = 0i32;
    let mut in_str = false;
    let mut escape = false;
    let mut end = None;
    for (i, c) in colon_rest.char_indices() {
        if escape {
            escape = false;
            continue;
        }
        if in_str {
            if c == '\\' {
                escape = true;
            } else if c == '"' {
                in_str = false;
            }
            continue;
        }
        match c {
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    end = Some(i + 1);
                    break;
                }
            }
            '"' => in_str = true,
            _ => {}
        }
    }

    let array_str = &colon_rest[..end?];
    serde_json::from_str::<Vec<String>>(array_str).ok()
}

// ── Connection test ───────────────────────────────────────────────────────────

/// Probe LM Studio's /models endpoint and return the list of loaded model IDs.
pub async fn fetch_models(base_url: &str) -> Result<Vec<String>> {
    let url = format!("{}/models", base_url.trim_end_matches('/'));

    let resp = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?
        .get(&url)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        bail!("LM Studio returned {status} — is it running at {base_url}?");
    }

    #[derive(Deserialize)]
    struct ModelsResponse {
        data: Vec<ModelObject>,
    }
    #[derive(Deserialize)]
    struct ModelObject {
        id: String,
    }

    let body = resp.text().await?;
    let parsed: ModelsResponse = serde_json::from_str(&body)
        .map_err(|e| anyhow::anyhow!("Unexpected response: {e}\n{body}"))?;

    Ok(parsed.data.into_iter().map(|m| m.id).collect())
}
