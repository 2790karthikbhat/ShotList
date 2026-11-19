// src/core/export.rs
// Export the reviewer's picks — copy selected images into a `Picks/` subfolder
// of the source directory.  Originals are never modified or moved.
//
// Filename collision handling
// ───────────────────────────
// Two picks can share the same filename when they come from different
// subdirectories (e.g. Card1/IMG_0001.ARW and Card2/IMG_0001.ARW).
// Strategy: on first occurrence use the bare filename; on collision prefix with
// the immediate parent folder name ("Card2_IMG_0001.ARW").  A numeric suffix
// (" (2)", " (3)" …) is appended if the prefixed name also collides.

use crate::core::image::CullState;
use crate::core::session::Session;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::{info, warn};

/// Result of an export run.
#[derive(Debug)]
pub struct ExportSummary {
    pub copied: usize,
    pub skipped: usize,
    pub dest: PathBuf,
}

/// Copy every image marked `Pick` into `{source_dir}/Picks/`.
pub fn export_picks(session: &Session) -> Result<ExportSummary> {
    let dest = session.source_dir.join("Picks");
    std::fs::create_dir_all(&dest)
        .with_context(|| format!("Could not create export folder {dest:?}"))?;

    let mut copied = 0;
    let mut skipped = 0;

    // Track which destination names have already been used this run so we can
    // detect collisions even when files don't exist on disk yet.
    let mut used_names: HashMap<String, usize> = HashMap::new();

    for id in &session.image_ids {
        let entry = &session.images[id];
        if entry.cull_state != CullState::Pick {
            continue;
        }

        let dest_name = resolve_dest_name(session, entry.filename(), &entry.path, &mut used_names);
        let target = dest.join(&dest_name);

        match std::fs::copy(&entry.path, &target) {
            Ok(_) => copied += 1,
            Err(e) => {
                warn!("Failed to copy {:?}: {e}", entry.path);
                skipped += 1;
            }
        }
    }

    info!("Exported {copied} pick(s) to {dest:?} ({skipped} skipped)");
    Ok(ExportSummary {
        copied,
        skipped,
        dest,
    })
}

/// Return a collision-free destination filename, updating `used` as a side effect.
fn resolve_dest_name(
    session: &Session,
    filename: &str,
    src_path: &std::path::Path,
    used: &mut HashMap<String, usize>,
) -> String {
    // First attempt: bare filename.
    if !used.contains_key(filename) {
        used.insert(filename.to_owned(), 1);
        return filename.to_owned();
    }

    // Collision: prefix with the immediate parent directory name.
    let parent_name = src_path
        .parent()
        .and_then(|p| p.strip_prefix(&session.source_dir).ok())
        .map(|rel| rel.to_string_lossy().replace(['/', '\\'], "_"))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "extra".to_owned());

    let prefixed = format!("{parent_name}_{filename}");

    if !used.contains_key(&prefixed) {
        used.insert(prefixed.clone(), 1);
        return prefixed;
    }

    // Still colliding: append a numeric suffix.
    let count = used.entry(prefixed.clone()).or_insert(1);
    *count += 1;
    let n = *count;
    let (stem, ext) = split_stem_ext(&prefixed);
    let final_name = if ext.is_empty() {
        format!("{stem} ({n})")
    } else {
        format!("{stem} ({n}).{ext}")
    };
    // Record the generated name so a future image with that exact stem cannot
    // silently overwrite it.
    used.insert(final_name.clone(), 1);
    final_name
}

fn split_stem_ext(name: &str) -> (&str, &str) {
    match name.rfind('.') {
        Some(i) if i > 0 => (&name[..i], &name[i + 1..]),
        _ => (name, ""),
    }
}
