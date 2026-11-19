// src/core/import.rs
// Folder import: scan a directory for supported image files and build a Session.
//
// This is pure scanning + filesystem metadata. Resuming previously-saved
// cull decisions is handled separately by `crate::db` (see `Db::sync_session`).

use crate::core::image::{ImageEntry, ImageFormat};
use crate::core::session::Session;
use anyhow::Result;
use chrono::{DateTime, Utc};
use std::path::{Path, PathBuf};
use tracing::{info, warn};
use walkdir::WalkDir;

/// Maximum directory depth to scan. A shoot is usually one folder, but many
/// photographers keep a couple of nesting levels (e.g. `Shoot/Card1/...`).
const MAX_DEPTH: usize = 4;

/// Recursively scan `dir` for supported image files and return a fresh,
/// unsorted list of [`ImageEntry`] values.
pub fn scan_images(dir: &Path) -> Vec<ImageEntry> {
    let mut entries = Vec::new();

    for dir_entry in WalkDir::new(dir)
        .max_depth(MAX_DEPTH)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if !dir_entry.file_type().is_file() {
            continue;
        }
        let path = dir_entry.path();

        // Skip the `Picks/` subfolder when scanning the *source* directory so
        // that re-importing the originals doesn't also pull in the export
        // copies.  We only skip it when it is a direct child of the scan root
        // — if the user explicitly opens the Picks folder itself we show every
        // image inside it normally.
        let is_picks_subdir = path
            .strip_prefix(dir)
            .ok()
            .and_then(|rel| rel.components().next())
            .is_some_and(|first| first.as_os_str().eq_ignore_ascii_case("Picks"));
        if is_picks_subdir {
            continue;
        }

        let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
            continue;
        };
        let Some(format) = ImageFormat::from_extension(ext) else {
            continue;
        };

        match build_entry(path.to_path_buf(), format) {
            Ok(entry) => entries.push(entry),
            Err(e) => warn!("Skipping {:?}: {e}", path),
        }
    }

    info!("Scanned {:?}: {} image(s) found", dir, entries.len());
    entries
}

/// Build a single [`ImageEntry`] from a path.
///
/// For RAW files, also runs a fast metadata-only EXIF read (no pixel decode)
/// so that `captured_at` is available immediately — this is what lets burst
/// grouping work at import time rather than waiting for thumbnail decodes.
fn build_entry(path: PathBuf, format: ImageFormat) -> Result<ImageEntry> {
    let meta = std::fs::metadata(&path)?;
    let file_size = meta.len();
    let modified_at: DateTime<Utc> = meta
        .modified()
        .map(DateTime::<Utc>::from)
        .unwrap_or_else(|_| Utc::now());

    let mut entry = ImageEntry::new(path.clone(), format, file_size, modified_at);

    // Fast metadata read for RAW files: fills captured_at + camera settings.
    // Cost: ~1–5 ms per file (header parse only, no demosaic).
    if format.is_raw() {
        if let Some(exif) = crate::core::exif::read_raw_exif(&path) {
            entry.captured_at = exif.captured_at;
            entry.iso = exif.iso;
            entry.shutter_speed = exif.shutter_speed;
            entry.aperture = exif.aperture;
            entry.focal_length_mm = exif.focal_length_mm;
            entry.lens_model = exif.lens_model;
        }
    }

    // Seed cull decisions from an existing XMP sidecar (written by culld on a
    // previous session, or by Affinity / Bridge / Lightroom).  The database
    // sync that follows import will overwrite this with culld's own DB record
    // if one exists, so culld's decisions always win over third-party XMP for
    // images that culld has already reviewed.
    let xmp_path = crate::core::xmp::sidecar_path(&path);
    if xmp_path.exists() {
        if let Some(xmp) = crate::core::xmp::read_sidecar(&xmp_path) {
            if xmp.cull_state != crate::core::image::CullState::Unreviewed || xmp.rating > 0 {
                entry.cull_state = xmp.cull_state;
                entry.rating = xmp.rating;
                entry.color_label = xmp.color_label;
            }
        }
    }

    Ok(entry)
}

/// Sort image entries into a stable, photographer-friendly order:
/// by capture time (falling back to file mtime), then by filename.
pub fn sort_entries(entries: &mut [ImageEntry]) {
    entries.sort_by(|a, b| {
        let a_time = a.captured_at.unwrap_or(a.modified_at);
        let b_time = b.captured_at.unwrap_or(b.modified_at);
        a_time
            .cmp(&b_time)
            .then_with(|| a.path.file_name().cmp(&b.path.file_name()))
    });
}

/// Scan a folder and assemble a fresh [`Session`] (no persistence applied).
pub fn session_from_folder(dir: PathBuf) -> Session {
    let name = dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Untitled")
        .to_string();

    let mut entries = scan_images(&dir);
    sort_entries(&mut entries);

    let mut session = Session::new(name, dir);
    for entry in entries {
        session.add_image(entry);
    }

    // Group consecutive burst frames now that captured_at is populated.
    crate::core::burst::assign_groups(&mut session);
    let burst_count = session
        .images
        .values()
        .filter(|e| e.group_id.is_some())
        .count();
    if burst_count > 0 {
        info!("{burst_count} image(s) assigned to burst groups");
    }

    session
}
