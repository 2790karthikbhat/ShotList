// src/core/image.rs
// Core domain types for a single image entry and its cull state machine.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

// ── Identity ──────────────────────────────────────────────────────────────────

/// Stable, opaque identifier for an image within a session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ImageId(Uuid);

impl ImageId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for ImageId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ImageId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::str::FromStr for ImageId {
    type Err = uuid::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.parse()?))
    }
}

// ── Cull state machine ────────────────────────────────────────────────────────

/// The reviewer's decision for a single image.
///
/// State transitions:
///   Unreviewed ──► Pick
///   Unreviewed ──► Reject
///   Unreviewed ──► Flagged
///   Pick       ──► Reject  (changed mind)
///   Reject     ──► Pick    (restored)
///   Any        ──► Unreviewed (reset)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum CullState {
    #[default]
    Unreviewed,
    Pick,
    Reject,
    Flagged,
}

impl CullState {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Unreviewed => "unreviewed",
            Self::Pick => "pick",
            Self::Reject => "reject",
            Self::Flagged => "flagged",
        }
    }

    /// Inverse of [`label`] — parse a stored label back into a state.
    /// Unknown strings fall back to `Unreviewed`.
    pub fn from_label(s: &str) -> Self {
        match s {
            "pick" => Self::Pick,
            "reject" => Self::Reject,
            "flagged" => Self::Flagged,
            _ => Self::Unreviewed,
        }
    }

    pub fn is_selected(&self) -> bool {
        matches!(self, Self::Pick | Self::Flagged)
    }
}

// ── Image formats ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImageFormat {
    // RAW formats
    Cr2,
    Cr3, // Canon
    Nef,
    Nrw, // Nikon
    Arw, // Sony
    Raf, // Fujifilm
    Rw2, // Panasonic
    Orf, // Olympus
    Dng, // Adobe DNG (also used by Leica, Ricoh, etc.)
    // Standard
    Jpeg,
    Tiff,
}

impl ImageFormat {
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "cr2" => Some(Self::Cr2),
            "cr3" => Some(Self::Cr3),
            "nef" => Some(Self::Nef),
            "nrw" => Some(Self::Nrw),
            "arw" => Some(Self::Arw),
            "raf" => Some(Self::Raf),
            "rw2" => Some(Self::Rw2),
            "orf" => Some(Self::Orf),
            "dng" => Some(Self::Dng),
            "jpg" | "jpeg" => Some(Self::Jpeg),
            "tif" | "tiff" => Some(Self::Tiff),
            _ => None,
        }
    }

    pub fn is_raw(&self) -> bool {
        !matches!(self, Self::Jpeg | Self::Tiff)
    }
}

// ── Exposure info ─────────────────────────────────────────────────────────────

/// Histogram-derived exposure signals computed from the thumbnail.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExposureInfo {
    /// Overall brightness (mean luma) in [0, 1].
    pub mean_luma: f32,
    /// Fraction of pixels with luma > 245/255 (blown highlights).
    pub blown_pct: f32,
    /// Fraction of pixels with luma < 10/255 (crushed shadows).
    pub crushed_pct: f32,
}

impl ExposureInfo {
    /// True if highlights are likely clipped (>2% of pixels blown).
    pub fn is_blown(&self) -> bool {
        self.blown_pct > 0.02
    }
    /// True if shadows are clipped (>5% of pixels crushed).
    pub fn is_crushed(&self) -> bool {
        self.crushed_pct > 0.05
    }
    /// True when the overall image looks underexposed (mean luma < 20%).
    pub fn is_underexposed(&self) -> bool {
        self.mean_luma < 0.20
    }
    /// True when the overall image looks overexposed  (mean luma > 80%).
    pub fn is_overexposed(&self) -> bool {
        self.mean_luma > 0.80
    }
}

// ── AI assessment scores ──────────────────────────────────────────────────────

/// Quality signals produced by local ONNX models or a remote AI provider.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AiAssessment {
    /// 0.0–1.0 — higher is sharper. Derived from Laplacian variance or a model.
    pub focus_score: Option<f32>,
    /// Detected face bounding boxes (x, y, w, h) in pixel coords.
    pub faces: Vec<FaceRegion>,
    /// Overall quality ranking relative to the scene (produced by AI provider).
    pub scene_rank: Option<f32>,
    /// Free-form label from the AI provider (e.g. "best expression").
    pub ai_label: Option<String>,
    /// Which provider produced this (for display and auditing).
    pub provider: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaceRegion {
    /// Normalised bounding box [0,1].
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    /// Probability that both eyes are open.
    pub eyes_open_prob: Option<f32>,
    /// Probability that the face is in focus.
    pub in_focus_prob: Option<f32>,
}

// ── The main image entry ──────────────────────────────────────────────────────

/// A single image file tracked within a culling session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageEntry {
    pub id: ImageId,

    /// Absolute path on disk.
    pub path: PathBuf,

    /// Parsed format from the file extension.
    pub format: ImageFormat,

    /// File size in bytes.
    pub file_size: u64,

    /// Last-modified timestamp from the filesystem.
    pub modified_at: DateTime<Utc>,

    /// EXIF capture timestamp (parsed lazily).
    pub captured_at: Option<DateTime<Utc>>,

    /// Camera-reported exposure settings (filled from EXIF during thumbnail decode).
    pub iso: Option<u32>,
    pub shutter_speed: Option<String>, // e.g. "1/500"
    pub aperture: Option<f32>,         // e.g. 2.8
    pub focal_length_mm: Option<f32>,  // e.g. 35.0
    pub lens_model: Option<String>,    // e.g. "FE 24-70mm F2.8 GM II"

    /// 64-bit difference hash of the thumbnail for near-duplicate detection.
    /// Two images with hamming distance ≤ 8 are considered near-duplicates.
    pub p_hash: Option<u64>,

    /// Histogram-derived exposure signals (filled during thumbnail decode).
    pub exposure: Option<ExposureInfo>,

    /// Burst group this image belongs to (None = standalone shot).
    /// Assigned by `burst::assign_groups` at import time.
    pub group_id: Option<u32>,

    /// Total frames in this burst group (duplicated per-entry for O(1) display).
    pub group_size: Option<u32>,

    /// Reviewer's decision.
    pub cull_state: CullState,

    /// Star rating 0–5 (0 = unrated).
    pub rating: u8,

    /// Colour label (for Lightroom-style colour tagging).
    pub color_label: Option<ColorLabel>,

    /// AI-derived quality scores.
    pub assessment: AiAssessment,

    /// When the entry was added to the session index.
    pub indexed_at: DateTime<Utc>,
}

impl ImageEntry {
    pub fn new(
        path: PathBuf,
        format: ImageFormat,
        file_size: u64,
        modified_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id: ImageId::new(),
            path,
            format,
            file_size,
            modified_at,
            captured_at: None,
            iso: None,
            shutter_speed: None,
            aperture: None,
            focal_length_mm: None,
            lens_model: None,
            p_hash: None,
            exposure: None,
            group_id: None,
            group_size: None,
            cull_state: CullState::default(),
            rating: 0,
            color_label: None,
            assessment: AiAssessment::default(),
            indexed_at: Utc::now(),
        }
    }

    /// Filename without directory path.
    pub fn filename(&self) -> &str {
        self.path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("<unknown>")
    }

    /// Human-readable file size.
    pub fn size_label(&self) -> String {
        let mb = self.file_size as f64 / 1_048_576.0;
        format!("{mb:.1} MB")
    }
}

// ── Colour labels ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ColorLabel {
    Red,
    Orange,
    Yellow,
    Green,
    Blue,
    Purple,
}
