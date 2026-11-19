// src/ai/mod.rs
// Pluggable AI provider system.
//
// The `AiProvider` trait is the single integration point. Any LLM/vision API
// that can score a photo implements it. Providers are selected at runtime
// based on which API key the user supplies in Settings.

pub mod anthropic;
pub mod autocull;
pub mod caption;
pub mod develop;
pub mod face_detector;
pub mod lmstudio;
pub mod local;
pub mod openai;
pub mod scoring;
pub mod sequence;

use crate::core::image::AiAssessment;
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

// ── Provider trait ────────────────────────────────────────────────────────────

/// A pluggable vision AI backend.
///
/// Each provider receives a JPEG-encoded image crop (or full image) and
/// returns structured quality signals.
#[async_trait]
pub trait AiProvider: Send + Sync {
    /// Human-readable name shown in the UI.
    fn name(&self) -> &'static str;

    /// Estimate the cost of one request in USD (shown to users before bulk runs).
    fn estimated_cost_usd(&self) -> f32 {
        0.0
    }

    /// Assess a single image.
    ///
    /// `jpeg_bytes` — a JPEG-encoded image (full or cropped to subject).
    /// Returns a partial `AiAssessment`; the caller merges it with any
    /// existing local assessment.
    async fn assess_image(&self, jpeg_bytes: &[u8]) -> Result<AiAssessment>;

    /// Batch assessment — default implementation calls assess_image sequentially.
    /// Providers with native batch endpoints should override this.
    async fn assess_batch(&self, images: Vec<Vec<u8>>) -> Vec<Result<AiAssessment>> {
        let mut results = Vec::with_capacity(images.len());
        for bytes in images {
            results.push(self.assess_image(&bytes).await);
        }
        results
    }
}

// ── Provider registry ─────────────────────────────────────────────────────────

/// Runtime registry of all configured providers.
pub struct ProviderRegistry {
    providers: Vec<Arc<dyn AiProvider>>,
    active_idx: usize,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
            active_idx: 0,
        }
    }

    pub fn register(&mut self, provider: Arc<dyn AiProvider>) {
        self.providers.push(provider);
    }

    pub fn active(&self) -> Option<Arc<dyn AiProvider>> {
        self.providers.get(self.active_idx).cloned()
    }

    pub fn set_active_by_name(&mut self, name: &str) -> bool {
        if let Some(idx) = self.providers.iter().position(|p| p.name() == name) {
            self.active_idx = idx;
            true
        } else {
            false
        }
    }

    pub fn provider_names(&self) -> Vec<&'static str> {
        self.providers.iter().map(|p| p.name()).collect()
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}
