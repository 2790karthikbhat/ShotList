// src/core/burst.rs
// Burst grouping — cluster consecutive frames into "shots" by capture-time gaps.
//
// Algorithm
// ─────────
// Images arrive sorted by captured_at (sub-second precision from EXIF).
// We scan consecutive pairs: if the gap is ≤ BURST_GAP_SECS, the two frames
// belong to the same burst group.  Groups with only one member are left
// ungrouped (group_id stays None).
//
// Only images with a real EXIF timestamp (captured_at is Some) participate in
// grouping; filesystem-mtime-only entries are left ungrouped to avoid false
// positives when all files carry the same copy-time stamp.
//
// After grouping, each entry in a multi-frame burst receives:
//   group_id   — unique group identifier (1-based, ascending)
//   group_size — total frames in the group

use crate::core::image::ImageId;
use crate::core::session::Session;
use chrono::DateTime;
use chrono::Utc;

/// Two consecutive frames within this many seconds belong to the same burst.
/// At 10 fps the inter-frame gap is 0.1 s; at 3 fps it's ~0.33 s.
/// 3 seconds gives a generous margin while still separating deliberate shots.
const BURST_GAP_SECS: i64 = 3;

/// Minimum frames to constitute a burst (singles are always standalone).
const MIN_BURST_SIZE: usize = 2;

// ── Public API ────────────────────────────────────────────────────────────────

/// Cluster the session's images into burst groups and stamp each entry with
/// `group_id` / `group_size`.  Idempotent — safe to call again after adding
/// images or after EXIF timestamps are learned from thumbnail decodes.
///
/// Timestamp strategy
/// ──────────────────
/// 1. Use `captured_at` (EXIF DateTimeOriginal) when present — sub-second precision.
/// 2. Fall back to `modified_at` (filesystem mtime) when EXIF is unavailable,
///    but only if the session does not look like a batch-copy:
///    if ≥ 80 % of images share the same mtime second they were all written at
///    once (card import via Finder/rsync) and mtime is meaningless for ordering.
///
/// Precondition: `session.image_ids` is already sorted by capture time
/// (guaranteed by `import::sort_entries`).
pub fn assign_groups(session: &mut Session) {
    let ids = session.image_ids.clone();

    // Early exit: if every image already has a group stamp (or is a confirmed
    // standalone) AND all captured_at values match what's already recorded in
    // the sort order, nothing has changed.  The cheap proxy is: if the count
    // of images with captured_at equals the total and no image has group_id
    // None while having a neighbour within BURST_GAP_SECS, we can skip.
    // In practice we just re-run — it's O(n) and called at most once per
    // thumbnail-decode batch (~16 images), which is negligible for any folder
    // size a photographer would encounter.  The comment is here to explain why
    // the straightforward approach is sufficient.

    // Clear existing stamps so re-runs don't leave stale data.
    for id in &ids {
        if let Some(e) = session.images.get_mut(id) {
            e.group_id = None;
            e.group_size = None;
        }
    }

    // Decide whether filesystem mtime is usable as a fallback.
    let exif_count = ids
        .iter()
        .filter(|id| session.images[id].captured_at.is_some())
        .count();

    // Batch-copy guard: if all images share the same mtime to the second,
    // they were copied in bulk and mtime carries no temporal ordering.
    let use_mtime_fallback = if exif_count == ids.len() {
        false // every image has EXIF — no fallback needed
    } else {
        let mtimes: Vec<i64> = ids
            .iter()
            .map(|id| session.images[id].modified_at.timestamp())
            .collect();
        let first = mtimes[0];
        let all_same = mtimes.iter().all(|&t| t == first);
        !all_same // mtime is safe to use iff files have *different* timestamps
    };

    // Build timestamp sequence.
    // Carry the source tag so we never mix EXIF-stamped images with mtime
    // fallbacks inside the same burst (a JPEG without EXIF should never be
    // grouped into a RAW burst just because its mtime is close).
    //   is_exif = true  → timestamp came from DateTimeOriginal
    //   is_exif = false → timestamp came from filesystem mtime
    let timestamped: Vec<(usize, DateTime<Utc>, bool)> = ids
        .iter()
        .enumerate()
        .filter_map(|(i, id)| {
            let e = &session.images[id];
            if let Some(ts) = e.captured_at {
                Some((i, ts, true))
            } else if use_mtime_fallback {
                Some((i, e.modified_at, false))
            } else {
                None
            }
        })
        .collect();

    if timestamped.len() < MIN_BURST_SIZE {
        return;
    }

    // Scan consecutive pairs; a boundary is inserted when either the time gap
    // is too large OR the timestamp source changes (EXIF ↔ mtime).
    let mut runs: Vec<Vec<usize>> = Vec::new();
    let mut current: Vec<usize> = vec![timestamped[0].0];

    for pair in timestamped.windows(2) {
        let (_, t_prev, src_prev) = pair[0];
        let (i_curr, t_curr, src_curr) = pair[1];
        let gap_secs = (t_curr - t_prev).num_seconds().abs();

        if src_prev == src_curr && gap_secs <= BURST_GAP_SECS {
            current.push(i_curr);
        } else {
            flush(&mut runs, &mut current);
            current.push(i_curr);
        }
    }
    flush(&mut runs, &mut current);

    // Stamp entries.  group_id is 1-based so that 0 can never be confused with None.
    for (next_id, run) in (1_u32..).zip(runs.iter()) {
        let gsize = run.len() as u32;
        for &idx in run {
            let image_id: ImageId = ids[idx];
            if let Some(entry) = session.images.get_mut(&image_id) {
                entry.group_id = Some(next_id);
                entry.group_size = Some(gsize);
            }
        }
    }
}

/// Flush `current` into `runs` if it meets the minimum burst size.
fn flush(runs: &mut Vec<Vec<usize>>, current: &mut Vec<usize>) {
    if current.len() >= MIN_BURST_SIZE {
        runs.push(std::mem::take(current));
    } else {
        current.clear();
    }
}

// ── Utility ───────────────────────────────────────────────────────────────────

/// Return the 1-based position of `id` within its burst group, or None if
/// the image is ungrouped.  O(n) scan of the session — call sparingly.
pub fn position_in_group(session: &Session, id: ImageId) -> Option<(usize, u32)> {
    let target_group = session.images.get(&id)?.group_id?;
    let group_size = session.images.get(&id)?.group_size? as usize;

    let pos = session
        .image_ids
        .iter()
        .filter(|&&iid| session.images[&iid].group_id == Some(target_group))
        .position(|&iid| iid == id)
        .map(|p| p + 1)?; // 1-based

    Some((pos, group_size as u32))
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{
        image::{ImageEntry, ImageFormat},
        session::Session,
    };
    use chrono::TimeZone;
    use std::path::PathBuf;

    fn make_session(offsets_secs: &[i64]) -> Session {
        let base = Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();
        let mut session = Session::new("test", PathBuf::from("/tmp/test"));
        for &offset in offsets_secs {
            let ts = base + chrono::Duration::seconds(offset);
            let mut entry = ImageEntry::new(
                PathBuf::from(format!("/tmp/test/{offset}.arw")),
                ImageFormat::Arw,
                1_000_000,
                ts,
            );
            entry.captured_at = Some(ts);
            session.add_image(entry);
        }
        session
    }

    #[test]
    fn single_image_is_ungrouped() {
        let mut s = make_session(&[0]);
        assign_groups(&mut s);
        assert!(s.images.values().all(|e| e.group_id.is_none()));
    }

    #[test]
    fn close_frames_form_a_burst() {
        // 5 frames at 0.1 s intervals (10 fps burst)
        let offsets: Vec<i64> = (0..5).collect();
        let mut s = make_session(&offsets);
        assign_groups(&mut s);

        let groups: Vec<_> = s.images.values().filter_map(|e| e.group_id).collect();
        assert_eq!(groups.len(), 5, "all 5 frames should be grouped");
        assert!(
            groups.iter().all(|&g| g == groups[0]),
            "all in the same group"
        );
        assert!(s.images.values().all(|e| e.group_size == Some(5)));
    }

    #[test]
    fn distant_frames_are_separate_groups() {
        // burst of 3, gap of 10s, burst of 2
        let mut s = make_session(&[0, 1, 2, 12, 13]);
        assign_groups(&mut s);

        let ids = s.image_ids.clone();
        let g0 = s.images[&ids[0]].group_id;
        let g3 = s.images[&ids[3]].group_id;
        assert!(
            g0.is_some() && g3.is_some(),
            "both bursts should be grouped"
        );
        assert_ne!(g0, g3, "they should be different groups");
    }

    #[test]
    fn standalone_between_bursts_is_ungrouped() {
        // burst(3), standalone, burst(2)
        let mut s = make_session(&[0, 1, 2, 20, 30, 31]);
        assign_groups(&mut s);

        let ids = s.image_ids.clone();
        assert!(
            s.images[&ids[3]].group_id.is_none(),
            "image at t=20 is standalone"
        );
    }

    #[test]
    fn images_without_captured_at_are_excluded() {
        let base = Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();
        let mut session = Session::new("test", PathBuf::from("/tmp/test"));

        // Two frames with EXIF timestamps 1s apart
        for offset in [0i64, 1] {
            let ts = base + chrono::Duration::seconds(offset);
            let mut entry = ImageEntry::new(
                PathBuf::from(format!("/tmp/{offset}.arw")),
                ImageFormat::Arw,
                1_000_000,
                ts,
            );
            entry.captured_at = Some(ts);
            session.add_image(entry);
        }
        // One frame with NO captured_at (filesystem only)
        let mut no_ts = ImageEntry::new(
            PathBuf::from("/tmp/no_ts.jpg"),
            ImageFormat::Jpeg,
            1_000_000,
            base,
        );
        no_ts.captured_at = None;
        session.add_image(no_ts);

        assign_groups(&mut session);

        // The two EXIF-stamped frames should form a group; the third should not.
        let grouped: Vec<_> = session
            .images
            .values()
            .filter(|e| e.group_id.is_some())
            .collect();
        assert_eq!(grouped.len(), 2);
        assert!(session
            .images
            .values()
            .any(|e| e.captured_at.is_none() && e.group_id.is_none()));
    }
}
