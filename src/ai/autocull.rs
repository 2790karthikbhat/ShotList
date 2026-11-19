// src/ai/autocull.rs
// Auto-cull proposal engine — reduces the reviewer's workload by automatically
// handling the obvious cases so they only have to judge the ambiguous ones.
//
// Rules (applied in order, only to Unreviewed images):
//
//   1. Technical failures → Reject
//      Any image that fails the hard quality floor:
//        • focus_score < FOCUS_FLOOR  (0.05)  — severely blurred / accidental shutter
//        • exposure.blown_pct > BLOWN_REJECT  (0.40)  — > 40 % of pixels blown
//        • exposure.mean_luma < LUMA_BLACK    (0.04)  — nearly black (lens cap, etc.)
//
//   2. Burst groups → best-of-burst
//      Within each group, pick the highest-scoring non-failure frame and
//      reject all others.  If every frame in a group is a failure the whole
//      group is already rejected by rule 1.
//
//   3. Everything still Unreviewed → leave for human review
//      Standalone shots (not in any burst) and single-survivor burst frames
//      are left untouched.
//
// Proposals are ordinary CullState changes — the user can override any
// individual image in the normal way.  Nothing is deleted.

use crate::ai::scoring::{composite_score, ScoringWeights};
use crate::core::image::{CullState, ImageId};
use crate::core::session::Session;
use std::collections::HashMap;

// ── Thresholds ────────────────────────────────────────────────────────────────

/// Focus scores below this are considered "trash" (accident / severe blur).
/// Calibrated against the 512-px Laplacian scorer: real photos ≥ 0.40 sharp.
const FOCUS_FLOOR: f32 = 0.05;

/// If more than this fraction of pixels is blown, the exposure is unusable.
const BLOWN_REJECT: f32 = 0.40;

/// Mean luma below this indicates an accidentally black / covered frame.
const LUMA_BLACK: f32 = 0.04;

// ── Result type ───────────────────────────────────────────────────────────────

/// What the engine did, returned to the caller for display.
#[derive(Debug, Default)]
pub struct AutoCullSummary {
    /// Images auto-rejected because they failed the quality floor.
    pub rejected_failures: usize,
    /// Number of burst groups processed (≥ 2 non-failure frames per group).
    pub burst_groups_processed: usize,
    /// Frames auto-picked as best-of-burst.
    pub burst_picks: usize,
    /// Frames auto-rejected as burst siblings.
    pub burst_rejects: usize,
    /// Images left Unreviewed for the human to judge.
    pub left_for_review: usize,
    /// Images skipped because signals (focus score) are not yet available.
    pub skipped_no_signals: usize,
}

impl AutoCullSummary {
    pub fn is_empty(&self) -> bool {
        self.rejected_failures == 0 && self.burst_groups_processed == 0
    }

    /// One-line human-readable summary for the status bar.
    pub fn status_line(&self) -> String {
        if self.is_empty() {
            return "Auto-cull: nothing to do — all images already reviewed or signals not ready."
                .into();
        }
        let mut parts = Vec::new();
        if self.rejected_failures > 0 {
            parts.push(format!("{} failure(s) rejected", self.rejected_failures));
        }
        if self.burst_groups_processed > 0 {
            parts.push(format!(
                "{} burst group(s) → {} picked, {} rejected",
                self.burst_groups_processed, self.burst_picks, self.burst_rejects
            ));
        }
        if self.left_for_review > 0 {
            parts.push(format!("{} left for review", self.left_for_review));
        }
        if self.skipped_no_signals > 0 {
            parts.push(format!(
                "{} skipped (thumbnails still loading)",
                self.skipped_no_signals
            ));
        }
        format!("Auto-cull: {}", parts.join(" · "))
    }
}

// ── Entry point ───────────────────────────────────────────────────────────────

/// Run the auto-cull proposal engine on `session`.
///
/// Returns the set of `(ImageId, new_state)` pairs so the caller can persist
/// them to the DB and write XMP sidecars without needing to diff the session.
pub fn propose(
    session: &mut Session,
    weights: &ScoringWeights,
) -> (AutoCullSummary, Vec<(ImageId, CullState)>) {
    let mut summary = AutoCullSummary::default();
    let mut changes: Vec<(ImageId, CullState)> = Vec::new();

    // ── Pass 1: Technical failures ────────────────────────────────────────────
    // Collect failures first; we need to know which images are "clean" before
    // doing burst processing.
    let mut is_failure: HashMap<ImageId, bool> = HashMap::new();

    for id in &session.image_ids {
        let entry = &session.images[id];
        if entry.cull_state != CullState::Unreviewed {
            is_failure.insert(*id, false);
            continue;
        }

        // Need at least a focus score to assess blur; skip if not ready yet.
        let Some(focus) = entry.assessment.focus_score else {
            summary.skipped_no_signals += 1;
            is_failure.insert(*id, false); // treat as non-failure (can't tell)
            continue;
        };

        let failure = is_technical_failure(focus, &entry.exposure);
        is_failure.insert(*id, failure);

        if failure {
            changes.push((*id, CullState::Reject));
            summary.rejected_failures += 1;
        }
    }

    // Apply failure rejections to the session so burst processing sees them.
    for &(id, state) in &changes {
        session.set_cull_state(id, state);
    }

    // ── Pass 2: Burst groups ──────────────────────────────────────────────────
    // Collect groups of still-Unreviewed non-failure images.
    let mut groups: HashMap<u32, Vec<ImageId>> = HashMap::new();
    for id in &session.image_ids {
        let entry = &session.images[id];
        if entry.cull_state != CullState::Unreviewed {
            continue;
        }
        if let Some(gid) = entry.group_id {
            // Only include images that aren't failures and have signals.
            if !is_failure.get(id).copied().unwrap_or(false)
                && entry.assessment.focus_score.is_some()
            {
                groups.entry(gid).or_default().push(*id);
            }
        }
    }

    for (_, mut members) in groups {
        if members.len() < 2 {
            // Single surviving frame — leave for human (can't compare against anything).
            continue;
        }

        // Score each member; higher = better keeper.
        members.sort_by(|a, b| {
            let sa = frame_score(&session.images[a], weights);
            let sb = frame_score(&session.images[b], weights);
            sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
        });

        let best = members[0];

        // Pick the best, reject the rest.
        changes.push((best, CullState::Pick));
        session.set_cull_state(best, CullState::Pick);
        summary.burst_picks += 1;

        for &reject_id in &members[1..] {
            changes.push((reject_id, CullState::Reject));
            session.set_cull_state(reject_id, CullState::Reject);
            summary.burst_rejects += 1;
        }

        summary.burst_groups_processed += 1;
    }

    // ── Count what's still unreviewed ─────────────────────────────────────────
    summary.left_for_review = session
        .images
        .values()
        .filter(|e| e.cull_state == CullState::Unreviewed)
        .count();

    (summary, changes)
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// True when the image quality is so bad it should be immediately discarded.
fn is_technical_failure(focus: f32, exposure: &Option<crate::core::image::ExposureInfo>) -> bool {
    if focus < FOCUS_FLOOR {
        return true;
    }
    if let Some(exp) = exposure {
        if exp.blown_pct > BLOWN_REJECT {
            return true;
        }
        if exp.mean_luma < LUMA_BLACK {
            return true;
        }
    }
    false
}

/// Quality score for burst-member ranking: composite score penalised for
/// exposure problems and boosted for having open eyes in the frame.
fn frame_score(entry: &crate::core::image::ImageEntry, weights: &ScoringWeights) -> f32 {
    let base = composite_score(&entry.assessment, weights);

    // Penalise exposure problems (blown highlights / crushed shadows).
    let exposure_penalty = entry
        .exposure
        .as_ref()
        .map(|e| e.blown_pct * 0.4 + e.crushed_pct * 0.2)
        .unwrap_or(0.0);

    (base - exposure_penalty).max(0.0)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{
        image::{AiAssessment, ExposureInfo, ImageEntry, ImageFormat},
        session::Session,
    };
    use chrono::{TimeZone, Utc};
    use std::path::PathBuf;

    fn entry_with_focus(
        path: &str,
        focus: f32,
        group: Option<u32>,
        gsize: Option<u32>,
    ) -> ImageEntry {
        let ts = Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();
        let mut e = ImageEntry::new(PathBuf::from(path), ImageFormat::Arw, 1_000, ts);
        e.captured_at = Some(ts);
        e.assessment = AiAssessment {
            focus_score: Some(focus),
            ..Default::default()
        };
        e.exposure = Some(ExposureInfo {
            mean_luma: 0.45,
            blown_pct: 0.01,
            crushed_pct: 0.01,
        });
        e.group_id = group;
        e.group_size = gsize;
        e
    }

    fn make_session(entries: Vec<ImageEntry>) -> Session {
        let mut s = Session::new("test", PathBuf::from("/tmp"));
        for e in entries {
            s.add_image(e);
        }
        s
    }

    #[test]
    fn technical_failure_is_rejected() {
        let mut session = make_session(vec![
            entry_with_focus("/tmp/blurry.arw", 0.02, None, None), // below FOCUS_FLOOR
            entry_with_focus("/tmp/sharp.arw", 0.60, None, None),
        ]);
        let (summary, changes) = propose(&mut session, &ScoringWeights::default());
        assert_eq!(summary.rejected_failures, 1);
        // Only the blurry image should have been changed
        let changed_paths: Vec<_> = changes
            .iter()
            .map(|(id, _)| session.images[id].path.to_str().unwrap().to_owned())
            .collect();
        assert!(changed_paths.iter().any(|p| p.contains("blurry")));
        assert!(!changed_paths.iter().any(|p| p.contains("sharp")));
    }

    #[test]
    fn burst_group_best_is_picked() {
        let entries = vec![
            entry_with_focus("/tmp/burst_a.arw", 0.80, Some(1), Some(3)),
            entry_with_focus("/tmp/burst_b.arw", 0.45, Some(1), Some(3)), // best
            entry_with_focus("/tmp/burst_c.arw", 0.30, Some(1), Some(3)),
        ];
        // Manually set focus so burst_b has highest score:
        let mut session = make_session(entries);
        // override burst_b's focus to be clearly highest
        for entry in session.images.values_mut() {
            if entry.path.to_str().unwrap().contains("burst_b") {
                entry.assessment.focus_score = Some(0.92);
            }
        }

        let (summary, _) = propose(&mut session, &ScoringWeights::default());
        assert_eq!(summary.burst_picks, 1, "one burst winner should be picked");
        assert_eq!(summary.burst_rejects, 2, "two siblings should be rejected");
        assert_eq!(summary.burst_groups_processed, 1);

        let burst_b_state = session
            .images
            .values()
            .find(|e| e.path.to_str().unwrap().contains("burst_b"))
            .unwrap()
            .cull_state;
        assert_eq!(burst_b_state, CullState::Pick);
    }

    #[test]
    fn already_reviewed_images_are_skipped() {
        let mut entries = vec![
            entry_with_focus("/tmp/already_picked.arw", 0.60, None, None),
            entry_with_focus("/tmp/failure.arw", 0.02, None, None),
        ];
        entries[0].cull_state = CullState::Pick; // already decided

        let mut session = make_session(entries);
        let (summary, changes) = propose(&mut session, &ScoringWeights::default());

        // already_picked must not be touched
        let picked_still_pick = session
            .images
            .values()
            .find(|e| e.path.to_str().unwrap().contains("already_picked"))
            .unwrap()
            .cull_state
            == CullState::Pick;
        assert!(
            picked_still_pick,
            "already-reviewed image must not be changed"
        );
        assert_eq!(summary.rejected_failures, 1);
        assert_eq!(changes.len(), 1);
    }

    #[test]
    fn standalone_clean_image_left_unreviewed() {
        let mut session = make_session(vec![entry_with_focus("/tmp/nice.arw", 0.70, None, None)]);
        let (summary, changes) = propose(&mut session, &ScoringWeights::default());
        assert!(
            changes.is_empty(),
            "standalone clean image should be left alone"
        );
        assert_eq!(summary.left_for_review, 1);
    }

    #[test]
    fn status_line_is_non_empty_after_changes() {
        let mut session = make_session(vec![
            entry_with_focus("/tmp/a.arw", 0.80, Some(1), Some(2)),
            entry_with_focus("/tmp/b.arw", 0.40, Some(1), Some(2)),
        ]);
        let (summary, _) = propose(&mut session, &ScoringWeights::default());
        let line = summary.status_line();
        assert!(line.contains("burst"), "{line}");
    }
}
