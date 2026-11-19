// src/core/session.rs
// A Session is one culling job: a folder of images + all reviewer decisions.

use crate::core::image::{CullState, ImageEntry, ImageId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(Uuid);

impl SessionId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Parse a stored UUID string back into a `SessionId`.
    pub fn parse(s: &str) -> Option<Self> {
        Uuid::parse_str(s).ok().map(Self)
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

// ── Session ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: SessionId,
    pub name: String,

    /// Root directory that was imported.
    pub source_dir: PathBuf,

    /// All images in import order.
    pub image_ids: Vec<ImageId>,

    /// Random-access map: id → entry.
    pub images: HashMap<ImageId, ImageEntry>,

    /// Index of the image currently shown in the viewer.
    pub cursor: usize,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Session {
    pub fn new(name: impl Into<String>, source_dir: PathBuf) -> Self {
        let now = Utc::now();
        Self {
            id: SessionId::new(),
            name: name.into(),
            source_dir,
            image_ids: Vec::new(),
            images: HashMap::new(),
            cursor: 0,
            created_at: now,
            updated_at: now,
        }
    }

    // ── Insertion ─────────────────────────────────────────────────────────

    pub fn add_image(&mut self, entry: ImageEntry) {
        let id = entry.id;
        self.image_ids.push(id);
        self.images.insert(id, entry);
        self.touch();
    }

    // ── Navigation ────────────────────────────────────────────────────────

    pub fn current_image(&self) -> Option<&ImageEntry> {
        self.image_ids
            .get(self.cursor)
            .and_then(|id| self.images.get(id))
    }

    pub fn current_image_mut(&mut self) -> Option<&mut ImageEntry> {
        if let Some(id) = self.image_ids.get(self.cursor).copied() {
            self.images.get_mut(&id)
        } else {
            None
        }
    }

    pub fn advance(&mut self) -> bool {
        if self.cursor + 1 < self.image_ids.len() {
            self.cursor += 1;
            true
        } else {
            false
        }
    }

    pub fn retreat(&mut self) -> bool {
        if self.cursor > 0 {
            self.cursor -= 1;
            true
        } else {
            false
        }
    }

    /// Advance to the next image that passes `filter`.  Falls back to the
    /// plain `advance` if no filter is active (all images visible).
    pub fn advance_visible(&mut self, filter: &SessionFilter) -> bool {
        let visible = self.visible_ids(filter);
        self.step_visible(&visible, 1)
    }

    /// Retreat to the previous image that passes `filter`.
    pub fn retreat_visible(&mut self, filter: &SessionFilter) -> bool {
        let visible = self.visible_ids(filter);
        self.step_visible(&visible, -1)
    }

    fn step_visible(&mut self, visible: &[ImageId], dir: i32) -> bool {
        if visible.is_empty() {
            return false;
        }
        let current = self.image_ids.get(self.cursor).copied();
        let vis_pos = current.and_then(|id| visible.iter().position(|&v| v == id));

        let next_pos = if let Some(vp) = vis_pos {
            // Normal: current image is visible — step in the requested direction.
            let np = (vp as i64 + dir as i64).clamp(0, (visible.len() as i64) - 1) as usize;
            if np == vp {
                return false; // Already at boundary.
            }
            np
        } else {
            // Current image was filtered out (e.g. just culled while a filter is active).
            // Jump to the nearest visible image in `dir` direction relative to the cursor.
            let cursor = self.cursor;
            let image_ids = &self.image_ids;
            if dir > 0 {
                visible
                    .iter()
                    .enumerate()
                    .find(|(_, &vid)| {
                        image_ids.iter().position(|&id| id == vid).unwrap_or(0) > cursor
                    })
                    .map(|(vi, _)| vi)
                    .unwrap_or(visible.len() - 1)
            } else {
                visible
                    .iter()
                    .enumerate()
                    .rev()
                    .find(|(_, &vid)| {
                        image_ids
                            .iter()
                            .position(|&id| id == vid)
                            .unwrap_or(usize::MAX)
                            < cursor
                    })
                    .map(|(vi, _)| vi)
                    .unwrap_or(0)
            }
        };

        let next_id = visible[next_pos];
        if let Some(idx) = self.image_ids.iter().position(|&id| id == next_id) {
            self.cursor = idx;
            true
        } else {
            false
        }
    }

    /// If the current image is hidden by `filter`, jump to the nearest visible one.
    pub fn clamp_to_visible(&mut self, filter: &SessionFilter) {
        let visible = self.visible_ids(filter);
        if visible.is_empty() {
            return;
        }
        let current = self.image_ids.get(self.cursor).copied();
        let still_visible = current.is_some_and(|id| visible.contains(&id));
        if !still_visible {
            // Jump to the visible image closest to current cursor position.
            let best = visible
                .iter()
                .min_by_key(|&&id| {
                    let pos = self.image_ids.iter().position(|&x| x == id).unwrap_or(0);
                    (pos as i64 - self.cursor as i64).unsigned_abs()
                })
                .copied();
            if let Some(id) = best {
                if let Some(idx) = self.image_ids.iter().position(|&x| x == id) {
                    self.cursor = idx;
                }
            }
        }
    }

    pub fn jump_to(&mut self, idx: usize) {
        self.cursor = idx.min(self.image_ids.len().saturating_sub(1));
    }

    // ── Cull operations ───────────────────────────────────────────────────

    pub fn set_cull_state(&mut self, id: ImageId, state: CullState) {
        if let Some(entry) = self.images.get_mut(&id) {
            entry.cull_state = state;
            self.touch();
        }
    }

    pub fn set_rating(&mut self, id: ImageId, rating: u8) {
        if let Some(entry) = self.images.get_mut(&id) {
            entry.rating = rating.min(5);
            self.touch();
        }
    }

    // ── Statistics ────────────────────────────────────────────────────────

    pub fn stats(&self) -> SessionStats {
        let mut stats = SessionStats {
            total: self.images.len(),
            ..Default::default()
        };
        for entry in self.images.values() {
            match entry.cull_state {
                CullState::Unreviewed => stats.unreviewed += 1,
                CullState::Pick => stats.picks += 1,
                CullState::Reject => stats.rejects += 1,
                CullState::Flagged => stats.flagged += 1,
            }
        }
        stats
    }

    // ── Sorted/filtered views ─────────────────────────────────────────────

    pub fn visible_ids(&self, filter: &SessionFilter) -> Vec<ImageId> {
        self.image_ids
            .iter()
            .filter(|id| {
                let e = &self.images[id];
                filter.allows(e)
            })
            .copied()
            .collect()
    }

    // ── Helpers ───────────────────────────────────────────────────────────

    fn touch(&mut self) {
        self.updated_at = Utc::now();
    }
}

// ── Stats ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Default)]
pub struct SessionStats {
    pub total: usize,
    pub unreviewed: usize,
    pub picks: usize,
    pub rejects: usize,
    pub flagged: usize,
}

impl SessionStats {
    pub fn reviewed(&self) -> usize {
        self.picks + self.rejects + self.flagged
    }

    pub fn progress_pct(&self) -> f32 {
        if self.total == 0 {
            return 0.0;
        }
        self.reviewed() as f32 / self.total as f32 * 100.0
    }
}

// ── Filter ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SessionFilter {
    pub show_unreviewed: bool,
    pub show_picks: bool,
    pub show_rejects: bool,
    pub show_flagged: bool,
    pub min_rating: u8,
}

impl SessionFilter {
    pub fn show_all() -> Self {
        Self {
            show_unreviewed: true,
            show_picks: true,
            show_rejects: true,
            show_flagged: true,
            min_rating: 0,
        }
    }

    pub fn allows(&self, entry: &ImageEntry) -> bool {
        let state_ok = match entry.cull_state {
            CullState::Unreviewed => self.show_unreviewed,
            CullState::Pick => self.show_picks,
            CullState::Reject => self.show_rejects,
            CullState::Flagged => self.show_flagged,
        };
        state_ok && entry.rating >= self.min_rating
    }
}
