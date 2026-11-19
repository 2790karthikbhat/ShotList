// src/ai/anthropic.rs
// AiProvider implementation backed by Anthropic Claude Vision (claude-3-5-sonnet).

use super::AiProvider;
use crate::core::image::{AiAssessment, FaceRegion};
use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::debug;

const API_URL: &str = "https://api.anthropic.com/v1/messages";
const MODEL: &str = "claude-opus-4-5";
const ANTHROPIC_VERSION: &str = "2023-06-01";

/// System prompt that instructs Claude to return structured JSON quality scores.
const SYSTEM_PROMPT: &str = r#"
You are a professional photo culling assistant. Given a photograph, respond ONLY
with a JSON object (no markdown, no prose) with these fields:

{
  "focus_score":  float 0.0–1.0  (1.0 = tack sharp, 0.0 = very blurry),
  "scene_rank":   float 0.0–1.0  (relative quality vs typical images in this scene),
  "label":        string         (e.g. "best expression", "eyes closed", "motion blur"),
  "faces": [
    {
      "eyes_open_prob":  float 0.0–1.0,
      "in_focus_prob":   float 0.0–1.0
    }
  ]
}

Be concise and consistent. If no faces are visible, return an empty faces array.
"#;

// ── Request / response shapes ─────────────────────────────────────────────────

#[derive(Serialize)]
struct AnthropicRequest<'a> {
    model: &'a str,
    max_tokens: u32,
    system: &'a str,
    messages: Vec<Message<'a>>,
}

#[derive(Serialize)]
struct Message<'a> {
    role: &'a str,
    content: Vec<ContentBlock>,
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ContentBlock {
    Image { source: ImageSource },
    Text { text: String },
}

#[derive(Serialize)]
struct ImageSource {
    #[serde(rename = "type")]
    kind: &'static str,
    media_type: &'static str,
    data: String, // base64
}

#[derive(Deserialize)]
struct AnthropicResponse {
    content: Vec<ResponseContent>,
}

#[derive(Deserialize)]
struct ResponseContent {
    #[serde(rename = "type")]
    kind: String,
    text: Option<String>,
}

/// Parsed inner JSON from Claude's response.
#[derive(Deserialize, Default)]
struct ScorePayload {
    focus_score: Option<f32>,
    scene_rank: Option<f32>,
    label: Option<String>,
    #[serde(default)]
    faces: Vec<FacePayload>,
}

#[derive(Deserialize)]
struct FacePayload {
    eyes_open_prob: Option<f32>,
    in_focus_prob: Option<f32>,
}

// ── Provider ──────────────────────────────────────────────────────────────────

pub struct AnthropicProvider {
    client: Client,
    api_key: String,
}

impl AnthropicProvider {
    pub fn new(api_key: impl Into<String>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to build HTTP client");

        Self {
            client,
            api_key: api_key.into(),
        }
    }
}

#[async_trait]
impl AiProvider for AnthropicProvider {
    fn name(&self) -> &'static str {
        "Claude Vision (Anthropic)"
    }

    fn estimated_cost_usd(&self) -> f32 {
        // Approximate cost for one image at ~500 input tokens + 100 output tokens
        // claude-3-5-sonnet: $3/$15 per 1M tokens → ~$0.002 per call
        0.002
    }

    async fn assess_image(&self, jpeg_bytes: &[u8]) -> Result<AiAssessment> {
        let b64 = B64.encode(jpeg_bytes);

        let body = AnthropicRequest {
            model: MODEL,
            max_tokens: 256,
            system: SYSTEM_PROMPT,
            messages: vec![Message {
                role: "user",
                content: vec![
                    ContentBlock::Image {
                        source: ImageSource {
                            kind: "base64",
                            media_type: "image/jpeg",
                            data: b64,
                        },
                    },
                    ContentBlock::Text {
                        text: "Assess this photograph.".into(),
                    },
                ],
            }],
        };

        let resp = self
            .client
            .post(API_URL)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .context("HTTP request to Anthropic API failed")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            bail!("Anthropic API error {status}: {text}");
        }

        let api_resp: AnthropicResponse =
            resp.json().await.context("Failed to parse API response")?;

        // Extract the text block.
        let raw_text = api_resp
            .content
            .into_iter()
            .find(|c| c.kind == "text")
            .and_then(|c| c.text)
            .unwrap_or_default();

        debug!("Anthropic raw response: {raw_text}");

        // Parse the JSON payload from Claude.
        let payload: ScorePayload = serde_json::from_str(raw_text.trim()).unwrap_or_default();

        let faces = payload
            .faces
            .into_iter()
            .map(|f| FaceRegion {
                // Bounding box unknown from text response — set to zeros.
                // Use local ONNX for bounding boxes, API for quality signals.
                x: 0.0,
                y: 0.0,
                width: 0.0,
                height: 0.0,
                eyes_open_prob: f.eyes_open_prob,
                in_focus_prob: f.in_focus_prob,
            })
            .collect();

        Ok(AiAssessment {
            focus_score: payload.focus_score,
            faces,
            scene_rank: payload.scene_rank,
            ai_label: payload.label,
            provider: Some(self.name().to_string()),
        })
    }
}
