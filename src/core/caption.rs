// src/core/caption.rs
// Caption data model — shared between DB, AI, commands, and XMP embedding.

use serde::{Deserialize, Serialize};

/// All editorial caption fields for one image.
///
/// Stored in the `captions` DB table and embedded into XMP sidecars.
/// Default is all-empty — an image has no caption until one is generated or
/// typed by the user.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Caption {
    /// Headline / short caption  (5–10 words, newspaper style).
    pub title: String,
    /// Body caption (2–3 sentences, journalistic present tense).
    pub description: String,
    /// Controlled vocabulary tags (8–15 specific + thematic terms).
    pub keywords: Vec<String>,
    /// "City, Country" where identifiable, otherwise empty.
    pub location: String,
    /// Photographer / agency credit line (user fills this in).
    pub credit: String,
}
