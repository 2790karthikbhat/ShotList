// src/commands/mod.rs
// All Tauri IPC command handlers, grouped by domain into sibling modules.
//
// Every public function decorated with #[tauri::command] is registered in
// main.rs (as `commands::fn_name`, re-exported below) and callable from the
// React frontend via `invoke(commandName, args)`.

use crate::core::session::SessionFilter;
use serde::{Deserialize, Serialize};

pub mod caption;
pub mod develop;
pub mod export;
pub mod folder;
pub mod session;
pub mod settings;
pub mod system;
pub mod zine;

pub use caption::*;
pub use develop::*;
pub use export::*;
pub use folder::*;
pub use session::*;
pub use settings::*;
pub use system::*;
pub use zine::*;

// ── DTO types sent to / received from the frontend ───────────────────────────

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SessionInfoDto {
    pub id: String,
    pub name: String,
    pub total: usize,
    pub picks: usize,
    pub rejects: usize,
    pub flagged: usize,
    pub unreviewed: usize,
    pub cursor: usize,
    pub current_image_id: Option<String>,
    pub progress_pct: f32,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ImageInfoDto {
    pub id: String,
    pub path: String,
    pub filename: String,
    pub cull_state: String,
    pub rating: u8,
    pub focus_score: Option<f32>,
    pub face_count: usize,
    pub group_id: Option<u32>,
    pub group_size: Option<u32>,
    pub iso: Option<u32>,
    pub shutter_speed: Option<String>,
    pub aperture: Option<f32>,
    pub focal_length_mm: Option<f32>,
    pub lens_model: Option<String>,
    pub size_label: String,
    pub exposure_blown: bool,
    pub exposure_crushed: bool,
    pub is_raw: bool,
    pub full_image_index: usize,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FilterPayload {
    pub show_unreviewed: bool,
    pub show_picks: bool,
    pub show_rejects: bool,
    pub show_flagged: bool,
    pub min_rating: u8,
}

impl From<FilterPayload> for SessionFilter {
    fn from(p: FilterPayload) -> Self {
        SessionFilter {
            show_unreviewed: p.show_unreviewed,
            show_picks: p.show_picks,
            show_rejects: p.show_rejects,
            show_flagged: p.show_flagged,
            min_rating: p.min_rating,
        }
    }
}

// ── Helpers shared across command domains ─────────────────────────────────────

pub(crate) fn session_dto(session: &crate::core::session::Session) -> SessionInfoDto {
    let stats = session.stats();
    SessionInfoDto {
        id: session.id.to_string(),
        name: session.name.clone(),
        total: stats.total,
        picks: stats.picks,
        rejects: stats.rejects,
        flagged: stats.flagged,
        unreviewed: stats.unreviewed,
        cursor: session.cursor,
        current_image_id: session.current_image().map(|e| e.id.to_string()),
        progress_pct: stats.progress_pct(),
    }
}

pub(crate) fn image_dto(entry: &crate::core::image::ImageEntry, full_index: usize) -> ImageInfoDto {
    let exposure = entry.exposure.as_ref();
    ImageInfoDto {
        id: entry.id.to_string(),
        path: entry.path.to_string_lossy().to_string(),
        filename: entry.filename().to_string(),
        cull_state: entry.cull_state.label().to_string(),
        rating: entry.rating,
        focus_score: entry.assessment.focus_score,
        face_count: entry.assessment.faces.len(),
        group_id: entry.group_id,
        group_size: entry.group_size,
        iso: entry.iso,
        shutter_speed: entry.shutter_speed.clone(),
        aperture: entry.aperture,
        focal_length_mm: entry.focal_length_mm,
        lens_model: entry.lens_model.clone(),
        size_label: entry.size_label(),
        exposure_blown: exposure.is_some_and(|e| e.is_blown()),
        exposure_crushed: exposure.is_some_and(|e| e.is_crushed()),
        is_raw: entry.format.is_raw(),
        full_image_index: full_index,
    }
}
