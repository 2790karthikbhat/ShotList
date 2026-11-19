// src/ai/develop.rs
// AI-powered auto-develop: streams parameter suggestions one-by-one.
//
// Supports Anthropic (Claude) and LM Studio (OpenAI-compatible local models).
// Suggestions arrive as Tauri `develop-suggestion` events.

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

// ── Shared types ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevelopSuggestion {
    pub param: String,
    pub value: f32,
    pub reason: String,
}

// ── System prompt (shared by both providers) ──────────────────────────────────

const SYSTEM: &str = r#"
You are an expert digital darkroom assistant helping a street photographer develop their images.

Analyse the photograph and the technical data provided, then suggest develop adjustments.
Emit ONE suggestion per line as NDJSON (no markdown, no prose, just one JSON object per line):

{"param":"<name>","value":<number>,"reason":"<short explanation, max 20 words>"}

Available parameters and their ranges:
  exposure   -5.0 to +5.0  (EV stops)
  contrast   -100 to +100
  highlights -100 to +100  (negative = recover blown highlights)
  shadows    -100 to +100  (positive = lift shadow detail)
  whites     -100 to +100
  blacks     -100 to +100
  clarity    -100 to +100  (local contrast / mid-frequency sharpening)
  vibrance   -100 to +100  (smart saturation, protects skin tones)
  saturation -100 to +100
  temperature -100 to +100 (negative = cooler, positive = warmer)
  tint       -100 to +100  (negative = magenta, positive = green)
  vignette   -100 to +100  (negative = darken edges, street photography classic)
  grain      0 to 100      (film grain)

Rules:
  • Only suggest parameters that need a real change. Skip ones that are fine.
  • Emit 4–8 suggestions in order of visual impact.
  • Keep reasons under 20 words — be direct, cite the technical data.
  • Favour the aesthetic of documentary / street photography.
"#;

// ── Shared helpers ────────────────────────────────────────────────────────────

fn build_technical_context(analysis: &crate::core::analysis::ImageAnalysis) -> String {
    format!(
        "Technical analysis:\n\
         • Mean luma: {:.0}%\n\
         • Blown highlights: {:.1}%\n\
         • Crushed shadows: {:.1}%\n\
         • Zone V (middle gray): {:.1}%\n\
         • Highlights (Zone VIII–X): {:.1}%\n\
         • Deep shadows (Zone 0–II): {:.1}%\n\
         {}",
        analysis.mean_luma * 100.0,
        analysis.blown_pct * 100.0,
        analysis.crushed_pct * 100.0,
        analysis.zone_pcts.get(5).unwrap_or(&0.0) * 100.0,
        (analysis.zone_pcts[8..].iter().sum::<f32>()) * 100.0,
        (analysis.zone_pcts[..3].iter().sum::<f32>()) * 100.0,
        if let Some(k) = analysis.color_temp_k {
            format!("• Estimated colour temperature: {} K", k)
        } else {
            "• Colour temperature: unable to estimate (high saturation)".into()
        },
    )
}

fn flush_suggestions(buf: &mut String, handle: &AppHandle) -> usize {
    let mut count = 0;
    while let Some(nl) = buf.find('\n') {
        let line = buf[..nl].trim().to_string();
        *buf = buf[nl + 1..].to_string();
        if line.is_empty() {
            continue;
        }
        if let Ok(s) = serde_json::from_str::<DevelopSuggestion>(&line) {
            let _ = handle.emit("develop-suggestion", &s);
            count += 1;
        }
    }
    count
}

// ── Anthropic implementation ──────────────────────────────────────────────────

#[cfg(feature = "provider-anthropic")]
mod anthropic_impl {
    use super::{build_technical_context, flush_suggestions, SYSTEM};
    use crate::core::analysis::ImageAnalysis;
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
        analysis: &ImageAnalysis,
        handle: &AppHandle,
    ) -> Result<()> {
        let technical_context = build_technical_context(analysis);
        let body = json!({
            "model":      MODEL,
            "max_tokens": 512,
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
                    {
                        "type": "text",
                        "text": format!("{}\n\nSuggest develop adjustments for this image.", technical_context),
                    }
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
                                        flush_suggestions(&mut text_buf, handle);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        text_buf.push('\n');
        flush_suggestions(&mut text_buf, handle);
        Ok(())
    }
}

#[cfg(feature = "provider-anthropic")]
pub use anthropic_impl::run;

// ── LM Studio implementation ──────────────────────────────────────────────────

// Single JSON-object format — more reliable with local models than NDJSON.
// `response_format: json_object` in the request forces JSON-mode output.
const SYSTEM_LMSTUDIO: &str = r#"
You are an expert digital darkroom assistant helping a street photographer develop their images.

Analyse the photograph and the technical data provided, then return ONLY a single valid JSON object.
No markdown, no code fences, no explanation.

The JSON object must have exactly this structure:
{"suggestions":[{"param":"<name>","value":<number>,"reason":"<max 20 words>"},...]}

Available parameters and their ranges:
  exposure   -5.0 to +5.0  (EV stops)
  contrast   -100 to +100
  highlights -100 to +100  (negative = recover blown highlights)
  shadows    -100 to +100  (positive = lift shadow detail)
  whites     -100 to +100
  blacks     -100 to +100
  clarity    -100 to +100  (local contrast)
  vibrance   -100 to +100
  saturation -100 to +100
  temperature -100 to +100 (negative = cooler, positive = warmer)
  tint       -100 to +100
  vignette   -100 to +100  (negative = darken edges)
  grain      0 to 100

Rules:
  • Only include parameters that need a real change (4–8 suggestions).
  • Keep reasons under 20 words.
  • Favour documentary / street photography aesthetics.
"#;

pub async fn run_lmstudio(
    base_url: &str,
    model: &str,
    jpeg_b64: &str,
    analysis: &crate::core::analysis::ImageAnalysis,
    handle: &AppHandle,
) -> anyhow::Result<()> {
    use crate::ai::lmstudio;
    use anyhow::bail;

    let technical_context = build_technical_context(analysis);
    let user_prompt = format!(
        "{}\n\nSuggest develop adjustments for this image.",
        technical_context
    );

    let body = lmstudio::build_vision_body(model, SYSTEM_LMSTUDIO, jpeg_b64, &user_prompt, 4096);

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

    let body = response.text().await?;
    tracing::info!("LM Studio develop raw response:\n{}", body);

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

    tracing::info!("LM Studio develop content:\n{}", full_text);

    // Find the first JSON object with a "suggestions" array and emit each entry.
    'parse: for obj in lmstudio::extract_json_objects(&full_text) {
        let Ok(val) = serde_json::from_str::<serde_json::Value>(&obj) else {
            continue;
        };
        let Some(arr) = val.get("suggestions").and_then(|v| v.as_array()) else {
            continue;
        };

        for item in arr {
            if let Ok(s) = serde_json::from_value::<DevelopSuggestion>(item.clone()) {
                let _ = handle.emit("develop-suggestion", &s);
            }
        }
        break 'parse;
    }

    Ok(())
}
