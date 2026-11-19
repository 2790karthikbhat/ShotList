// src/commands/export.rs
// Picks export (plain copy) and Developed export (apply develop params, write JPEG).

use crate::core::image::ImageId;
use crate::state::AppState;
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

#[tauri::command]
pub async fn export_picks(state: State<'_, AppState>) -> Result<String, String> {
    let session = state.session.lock().unwrap();
    let session = session.as_ref().ok_or("no session")?;

    match crate::core::export::export_picks(session) {
        Ok(summary) => Ok(format!(
            "Exported {} pick(s) → {}",
            summary.copied,
            summary.dest.display()
        )),
        Err(e) => Err(e.to_string()),
    }
}

// ── Develop export ────────────────────────────────────────────────────────────

/// Export payload emitted during batch export so the frontend can show progress.
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ExportProgress {
    pub done: usize,
    pub total: usize,
    pub filename: String,
}

/// Core export helper: loads the image, applies develop params, writes JPEG.
/// Returns the output path on success.
fn export_one(
    entry: &crate::core::image::ImageEntry,
    params: &crate::core::develop::DevelopParams,
    quality: u8,
    source_dir: &std::path::Path,
    db: Option<&crate::db::Db>,
    viewer_dir: &std::path::Path,
    thumb_dir: &std::path::Path,
) -> anyhow::Result<std::path::PathBuf> {
    use crate::core::develop::apply_develop;
    use image::ImageEncoder as _;

    // ── Load image ────────────────────────────────────────────────────────────
    let img = if entry.format.is_raw() {
        // For RAW: use HD viewer cache (already the embedded full-res preview).
        let id = entry.id.to_string();
        let hd = viewer_dir.join(format!("{id}_hd.jpg"));
        let thumb = thumb_dir.join(format!("{id}.jpg"));
        if hd.exists() {
            image::open(&hd)?
        } else if thumb.exists() {
            image::open(&thumb)?
        } else {
            anyhow::bail!(
                "No decoded image cached for {} — view it first",
                entry.filename()
            );
        }
    } else {
        image::open(&entry.path)?
    };

    // ── Apply develop ─────────────────────────────────────────────────────────
    let processed = apply_develop(&img, params);
    let rgb = processed.to_rgb8();

    // ── Output path: <source_dir>/Developed/<stem>.jpg ─────────────────────
    let developed_dir = source_dir.join("Developed");
    std::fs::create_dir_all(&developed_dir)?;

    let stem = entry.path.file_stem().unwrap_or_default().to_string_lossy();
    let out = developed_dir.join(format!("{stem}.jpg"));

    let mut jpeg: Vec<u8> = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, quality).write_image(
        rgb.as_raw(),
        rgb.width(),
        rgb.height(),
        image::ExtendedColorType::Rgb8,
    )?;
    std::fs::write(&out, &jpeg)?;

    // ── XMP sidecar with caption (if one exists) ──────────────────────────────
    if let Some(db) = db {
        let id_str = entry.id.to_string();
        if let Ok(Some(caption)) = crate::db::queries::load_caption(&db.conn, &id_str) {
            // Write XMP next to the exported JPEG.
            let xmp_path = out.with_extension("xmp");
            if let Ok(xml) = std::fs::read_to_string(crate::core::xmp::sidecar_path(&entry.path)) {
                // If original XMP exists, write a new one alongside the JPEG.
                let _ = xml; // we re-render from struct
            }
            let fake_entry = crate::core::image::ImageEntry {
                path: out.clone(),
                ..entry.clone()
            };
            let _ = crate::core::xmp::write_sidecar_with_caption(&fake_entry, &caption);
            let _ = xmp_path; // path is computed inside write_sidecar_with_caption
        }
    }

    Ok(out)
}

/// Export the current image with the provided (possibly unsaved) develop params.
/// `params` comes straight from the UI so unsaved slider positions are respected.
#[tauri::command]
pub fn export_developed(
    state: State<'_, AppState>,
    image_id: String,
    params: crate::core::develop::DevelopParams,
    quality: u8,
) -> Result<String, String> {
    let id: ImageId = image_id.parse().map_err(|e: uuid::Error| e.to_string())?;

    let (entry, source_dir) = {
        let session_guard = state.session.lock().unwrap();
        let session = session_guard.as_ref().ok_or("no session")?;
        let entry = session.images.get(&id).ok_or("image not found")?.clone();
        let source_dir = session.source_dir.clone();
        (entry, source_dir)
    };

    let db_guard = state.db.lock().unwrap();
    let db = db_guard.as_ref();

    let out = export_one(
        &entry,
        &params,
        quality.clamp(50, 100),
        &source_dir,
        db,
        &state.viewer_cache_dir,
        &state.thumb_cache_dir,
    )
    .map_err(|e| e.to_string())?;

    Ok(format!("Exported → {}", out.display()))
}

/// Export ALL picks with their DB-saved develop params (defaults if never saved).
/// Emits `export-progress` events so the frontend can show a progress bar.
#[tauri::command]
pub async fn export_all_picks(
    state: State<'_, AppState>,
    quality: u8,
    handle: AppHandle,
) -> Result<String, String> {
    use crate::core::image::CullState;

    // Collect what we need from the locked state before going async.
    let (picks, source_dir, viewer_dir, thumb_dir) = {
        let session_guard = state.session.lock().unwrap();
        let session = session_guard.as_ref().ok_or("no session")?;

        let picks: Vec<_> = session
            .image_ids
            .iter()
            .filter_map(|id| session.images.get(id))
            .filter(|e| e.cull_state == CullState::Pick)
            .cloned()
            .collect();

        (
            picks,
            session.source_dir.clone(),
            state.viewer_cache_dir.clone(),
            state.thumb_cache_dir.clone(),
        )
    };

    if picks.is_empty() {
        return Err("No picks to export — mark some images as picks first".into());
    }

    let total = picks.len();
    let quality = quality.clamp(50, 100);
    let mut exported = 0usize;
    let mut skipped = 0usize;

    for (i, entry) in picks.iter().enumerate() {
        let id_str = entry.id.to_string();

        // Load develop params from DB, fall back to identity defaults.
        let params = {
            let db_guard = state.db.lock().unwrap();
            if let Some(db) = db_guard.as_ref() {
                crate::db::queries::load_develop_params(&db.conn, &id_str)
                    .ok()
                    .flatten()
                    .unwrap_or_default()
            } else {
                crate::core::develop::DevelopParams::default()
            }
        };

        let db_guard = state.db.lock().unwrap();
        let db = db_guard.as_ref();

        match export_one(
            entry,
            &params,
            quality,
            &source_dir,
            db,
            &viewer_dir,
            &thumb_dir,
        ) {
            Ok(_) => exported += 1,
            Err(e) => {
                tracing::warn!("Export failed for {}: {e}", entry.filename());
                skipped += 1;
            }
        }
        drop(db_guard);

        let _ = handle.emit(
            "export-progress",
            ExportProgress {
                done: i + 1,
                total,
                filename: entry.filename().to_string(),
            },
        );
    }

    let dest = source_dir.join("Developed");
    Ok(format!(
        "Exported {exported} pick(s) → {}{skipped_note}",
        dest.display(),
        skipped_note = if skipped > 0 {
            format!(" ({skipped} skipped — view them first)")
        } else {
            String::new()
        },
    ))
}
