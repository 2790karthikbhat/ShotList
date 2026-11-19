// src/ai/openai.rs
// AiProvider implementation backed by OpenAI GPT-4o Vision.
// Mirrors the Anthropic provider — swap API shape as needed.

use super::AiProvider;
use crate::core::image::{AiAssessment, FaceRegion};
use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::debug;

const API_URL: &str = "https://api.openai.com/v1/chat/completions";
const MODEL: &str = "gpt-4o";

const SYSTEM_PROMPT: &str = r#"
You are a professional photo culling assistant. Given a photograph, respond ONLY
with a JSON object (no markdown) with these fields:
{
  "focus_score": float 0.0-1.0,
  "scene_rank":  float 0.0-1.0,
  "label":       string,
  "faces": [{ "eyes_open_prob": float, "in_focus_prob": float }]
}
"#;

#[derive(Serialize)]
struct OaiRequest<'a> {
    model: &'a str,
    max_tokens: u32,
    messages: Vec<OaiMessage>,
}

#[derive(Serialize)]
struct OaiMessage {
    role: String,
    content: Vec<OaiContent>,
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum OaiContent {
    Text { text: String },
    ImageUrl { image_url: OaiImageUrl },
}

#[derive(Serialize)]
struct OaiImageUrl {
    url: String, // "data:image/jpeg;base64,..."
}

#[derive(Deserialize)]
struct OaiResponse {
    choices: Vec<OaiChoice>,
}

#[derive(Deserialize)]
struct OaiChoice {
    message: OaiRespMessage,
}

#[derive(Deserialize)]
struct OaiRespMessage {
    content: String,
}

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

pub struct OpenAiProvider {
    client: Client,
    api_key: String,
}

impl OpenAiProvider {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("HTTP client build failed"),
            api_key: api_key.into(),
        }
    }
}

#[async_trait]
impl AiProvider for OpenAiProvider {
    fn name(&self) -> &'static str {
        "GPT-4o Vision (OpenAI)"
    }

    fn estimated_cost_usd(&self) -> f32 {
        0.003 // rough estimate per image call
    }

    async fn assess_image(&self, jpeg_bytes: &[u8]) -> Result<AiAssessment> {
        let data_url = format!("data:image/jpeg;base64,{}", B64.encode(jpeg_bytes));

        let body = OaiRequest {
            model: MODEL,
            max_tokens: 256,
            messages: vec![
                OaiMessage {
                    role: "system".into(),
                    content: vec![OaiContent::Text {
                        text: SYSTEM_PROMPT.into(),
                    }],
                },
                OaiMessage {
                    role: "user".into(),
                    content: vec![
                        OaiContent::ImageUrl {
                            image_url: OaiImageUrl { url: data_url },
                        },
                        OaiContent::Text {
                            text: "Assess this photograph.".into(),
                        },
                    ],
                },
            ],
        };

        let resp = self
            .client
            .post(API_URL)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .context("HTTP request to OpenAI API failed")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            bail!("OpenAI API error {status}: {text}");
        }

        let api_resp: OaiResponse = resp.json().await?;
        let raw = api_resp
            .choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .unwrap_or_default();

        debug!("OpenAI raw response: {raw}");

        let payload: ScorePayload = serde_json::from_str(raw.trim()).unwrap_or_default();

        let faces = payload
            .faces
            .into_iter()
            .map(|f| FaceRegion {
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
