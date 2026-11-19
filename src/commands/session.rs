// src/commands/session.rs
// Session lookup, image listing, navigation, viewer image source, cull
// decisions, thumbnails, auto-cull, technical analysis, and filtering.

use super::{image_dto, session_dto, FilterPayload, ImageInfoDto, SessionInfoDto};
use crate::core::{
    image::{CullState, ImageId},
    session::SessionFilter,
    thumbnailer::ThumbRequest,
};
use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub fn get_session(state: State<'_, AppState>) -> Result<Option<SessionInfoDto>, String> {
    let session = state.session.lock().unwrap();
    Ok(session.as_ref().map(session_dto))
}

// ── Image listing ─────────────────────────────────────────────────────────────

/// Returns the visible images under the current filter (ordered for the filmstrip).
#[tauri::command]
pub fn get_visible_images(state: State<'_, AppState>) -> Result<Vec<ImageInfoDto>, String> {
    let filter = state.filter.lock().unwrap().clone();
    let session = state.session.lock().unwrap();
    let session = session.as_ref().ok_or("no session")?;

    let visible = session.visible_ids(&filter);
    Ok(visible
        .iter()
        .map(|id| {
            let idx = session
                .image_ids
                .iter()
                .position(|&x| x == *id)
                .unwrap_or(0);
            image_dto(&session.images[id], idx)
        })
        .collect())
}

#[tauri::command]
pub fn get_current_image(state: State<'_, AppState>) -> Result<Option<ImageInfoDto>, String> {
    let session = state.session.lock().unwrap();
    let session = session.as_ref().ok_or("no session")?;
    Ok(session
        .current_image()
        .map(|e| image_dto(e, session.cursor)))
}

// ── Navigation ────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn navigate(state: State<'_, AppState>, direction: String) -> Result<Option<String>, String> {
    let filter = state.filter.lock().unwrap().clone();
    let mut session = state.session.lock().unwrap();
    let session = session.as_mut().ok_or("no session")?;

    match direction.as_str() {
        "next" => {
            session.advance_visible(&filter);
        }
        "prev" => {
            session.retreat_visible(&filter);
        }
        other => return Err(format!("unknown direction: {other}")),
    }

    // Enqueue high-res viewer decode for RAW images.
    if let Some(entry) = session.current_image() {
        if entry.format.is_raw() {
            let _ = state.thumb_request_tx.try_send(ThumbRequest {
                id: entry.id,
                path: entry.path.clone(),
                format: entry.format,
                high_res: true,
            });
        }
    }

    Ok(session.current_image().map(|e| e.id.to_string()))
}

#[tauri::command]
pub fn jump_to_image(state: State<'_, AppState>, index: usize) -> Result<(), String> {
    let mut session = state.session.lock().unwrap();
    let session = session.as_mut().ok_or("no session")?;
    session.jump_to(index);

    if let Some(entry) = session.current_image() {
        if entry.format.is_raw() {
            let _ = state.thumb_request_tx.try_send(ThumbRequest {
                id: entry.id,
                path: entry.path.clone(),
                format: entry.format,
                high_res: true,
            });
        }
    }
    Ok(())
}

// ── Viewer image source ───────────────────────────────────────────────────────

/// Returns a `data:image/...;base64,...` URL ready to set as an <img> src.
/// For JPEG/PNG/TIFF: reads the original file and returns it as a data URL.
/// For RAW: returns the cached HD JPEG as a data URL if it exists, otherwise
///          triggers a high-res decode and returns "" so the frontend can wait
///          for the thumbnail-ready(highRes=true) event.
#[tauri::command]
pub fn get_image_src(
    state: State<'_, AppState>,
    image_id: String,
    pristine: Option<bool>,
) -> Result<String, String> {
    use base64::Engine as _;

    let id: ImageId = image_id.parse().map_err(|e: uuid::Error| e.to_string())?;
    let session = state.session.lock().unwrap();
    let session = session.as_ref().ok_or("no session")?;
    let entry = session.images.get(&id).ok_or("image not found")?;

    // Prefer the geometry-adjusted "developed" cache unless the caller
    // explicitly wants the true original (Develop stage's "Before" toggle).
    if pristine != Some(true) {
        let dev = state
            .viewer_cache_dir
            .join(format!("{image_id}_hd_dev.jpg"));
        if dev.exists() {
            let bytes = std::fs::read(&dev).map_err(|e| e.to_string())?;
            let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
            return Ok(format!("data:image/jpeg;base64,{b64}"));
        }
    }

    if !entry.format.is_raw() {
        // For JPEG/PNG/TIFF, read and base64-encode the original file.
        let bytes = std::fs::read(&entry.path).map_err(|e| format!("read error: {e}"))?;
        let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
        let mime = match entry.format {
            crate::core::image::ImageFormat::Jpeg => "image/jpeg",
            crate::core::image::ImageFormat::Tiff => "image/tiff",
            _ => "image/jpeg", // shouldn't reach here
        };
        return Ok(format!("data:{mime};base64,{b64}"));
    }

    // RAW: return cached viewer JPEG as data URL if available.
    let cached = state.viewer_cache_dir.join(format!("{image_id}_hd.jpg"));
    if cached.exists() {
        let bytes = std::fs::read(&cached).map_err(|e| e.to_string())?;
        let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
        return Ok(format!("data:image/jpeg;base64,{b64}"));
    }

    // Trigger a high-res decode; frontend listens for thumbnail-ready(highRes=true).
    if let Err(e) = state.thumb_request_tx.try_send(ThumbRequest {
        id: entry.id,
        path: entry.path.clone(),
        format: entry.format,
        high_res: true,
    }) {
        // The viewer will be stuck on "Loading…" forever if this silently
        // fails — no thumbnail-ready event will ever arrive for it.
        tracing::warn!("viewer high-res decode not queued for {image_id}: {e}");
    }
    Ok(String::new()) // "" = still loading, event will follow
}

// ── Cull decisions ────────────────────────────────────────────────────────────

/// Set the cull state for an image and persist to DB + XMP sidecar.
/// Returns updated session stats so the toolbar can refresh in one round-trip.
#[tauri::command]
pub fn set_cull_state(
    state: State<'_, AppState>,
    path: String,
    cull_state: String,
) -> Result<SessionInfoDto, String> {
    let new_state = CullState::from_label(&cull_state);

    let mut session = state.session.lock().unwrap();
    let session = session.as_mut().ok_or("no session")?;

    // Find the image by path.
    let id = session
        .images
        .values()
        .find(|e| e.path.to_string_lossy() == path)
        .map(|e| e.id)
        .ok_or_else(|| format!("image not found: {path}"))?;

    session.set_cull_state(id, new_state);

    // Auto-advance after decision.
    let filter = state.filter.lock().unwrap().clone();
    session.advance_visible(&filter);

    // Persist.
    let session_id = session.id.to_string();
    if let Some(db) = state.db.lock().unwrap().as_ref() {
        let _ = crate::db::queries::update_cull_state(&db.conn, &session_id, &path, new_state);
    }

    // Write XMP sidecar (non-fatal).
    if let Some(entry) = session
        .images
        .values()
        .find(|e| e.path.to_string_lossy() == path)
    {
        let _ = crate::core::xmp::write_sidecar(entry);
    }

    Ok(session_dto(session))
}

#[tauri::command]
pub fn set_rating(state: State<'_, AppState>, path: String, rating: u8) -> Result<(), String> {
    let mut session = state.session.lock().unwrap();
    let session = session.as_mut().ok_or("no session")?;

    let id = session
        .images
        .values()
        .find(|e| e.path.to_string_lossy() == path)
        .map(|e| e.id)
        .ok_or_else(|| format!("image not found: {path}"))?;

    session.set_rating(id, rating);

    let session_id = session.id.to_string();
    if let Some(db) = state.db.lock().unwrap().as_ref() {
        let _ = crate::db::queries::update_rating(&db.conn, &session_id, &path, rating);
    }
    Ok(())
}

// ── Thumbnails ────────────────────────────────────────────────────────────────

/// Request filmstrip thumbnail decodes for a batch of image IDs.
/// Results arrive asynchronously via the `thumbnail-ready` event.
#[tauri::command]
pub fn request_thumbnails(
    state: State<'_, AppState>,
    image_ids: Vec<String>,
) -> Result<(), String> {
    let session = state.session.lock().unwrap();
    let Some(session) = session.as_ref() else {
        tracing::warn!("request_thumbnails called but no session");
        return Ok(());
    };

    let mut queued = 0usize;
    let mut not_found = 0usize;
    for id_str in &image_ids {
        let Ok(id) = id_str.parse::<ImageId>() else {
            continue;
        };
        let Some(entry) = session.images.get(&id) else {
            not_found += 1;
            continue;
        };
        match state.thumb_request_tx.try_send(ThumbRequest {
            id: entry.id,
            path: entry.path.clone(),
            format: entry.format,
            high_res: false,
        }) {
            Ok(_) => queued += 1,
            Err(e) => tracing::warn!("thumb channel full/closed: {e}"),
        }
    }
    tracing::info!(
        "request_thumbnails: queued={queued} not_found={not_found} of {} ids",
        image_ids.len()
    );
    Ok(())
}

/// Return base64 data-URL thumbnails for a list of image IDs by reading the
/// on-disk thumb/viewer cache.  Falls back gracefully if a file is absent.
/// This bypasses the async thumbnailer event flow, so the caller gets results
/// immediately without waiting for thumbnail-ready events.
#[tauri::command]
pub fn get_zine_thumb_urls(
    state: State<'_, AppState>,
    image_ids: Vec<String>,
) -> Result<std::collections::HashMap<String, String>, String> {
    use base64::Engine as _;

    let thumb_dir = &state.thumb_cache_dir;
    let viewer_dir = &state.viewer_cache_dir;
    let mut out = std::collections::HashMap::with_capacity(image_ids.len());

    for id in &image_ids {
        let dev_thumb_path = thumb_dir.join(format!("{id}_dev.jpg"));
        let dev_hd_path = viewer_dir.join(format!("{id}_hd_dev.jpg"));
        let hd_path = viewer_dir.join(format!("{id}_hd.jpg"));
        let thumb_path = thumb_dir.join(format!("{id}.jpg"));

        let path = if dev_thumb_path.exists() {
            dev_thumb_path
        } else if dev_hd_path.exists() {
            dev_hd_path
        } else if thumb_path.exists() {
            thumb_path
        } else if hd_path.exists() {
            hd_path
        } else {
            continue;
        };

        if let Ok(bytes) = std::fs::read(&path) {
            let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
            out.insert(id.clone(), format!("data:image/jpeg;base64,{b64}"));
        }
    }

    tracing::info!(
        "get_zine_thumb_urls: returned {}/{} thumbnails from cache",
        out.len(),
        image_ids.len()
    );
    Ok(out)
}

#[tauri::command]
pub fn run_auto_cull(state: State<'_, AppState>) -> Result<String, String> {
    use crate::ai::{autocull, scoring::ScoringWeights};

    let mut session = state.session.lock().unwrap();
    let session = session.as_mut().ok_or("no session")?;

    let total = session.images.len();
    let scored = session
        .images
        .values()
        .filter(|e| e.assessment.focus_score.is_some())
        .count();

    if scored == 0 {
        return Err(format!(
            "No thumbnails decoded yet ({total} pending) — wait a moment and try again"
        ));
    }

    // Warn if fewer than half are scored (auto-cull will still run on what's available).
    let _partial = scored < total / 2;

    let (summary, changes) = autocull::propose(session, &ScoringWeights::default());

    if let Some(db) = state.db.lock().unwrap().as_ref() {
        let sid = session.id.to_string();
        for (id, state) in &changes {
            let path = session.images[id].path.to_string_lossy().to_string();
            let _ = crate::db::queries::update_cull_state(&db.conn, &sid, &path, *state);
        }
    }

    Ok(summary.status_line())
}

// ── Technical Analyser ────────────────────────────────────────────────────────

/// Compute technical analysis signals for a single image.
///
/// Reads the cached thumbnail JPEG from disk (written by the thumbnail emitter)
/// and runs histogram, zone-system, focus-heatmap, and colour-temp analysis on
/// it — typically < 30 ms.  Returns an error if the thumbnail is not yet cached
/// (the image must have been viewed in the filmstrip at least once).
#[tauri::command]
pub fn get_image_analysis(
    state: State<'_, AppState>,
    image_id: String,
) -> Result<crate::core::analysis::ImageAnalysis, String> {
    use crate::core::analysis;

    let id: ImageId = image_id.parse().map_err(|e| format!("bad id: {e}"))?;

    // Try filmstrip-quality thumbnail first (fastest, always sufficient for analysis).
    let thumb_path = state.thumb_cache_dir.join(format!("{id}.jpg"));
    // Fall back to hi-res viewer cache if the thumbnail hasn't been written yet.
    let viewer_path = state.viewer_cache_dir.join(format!("{id}_hd.jpg"));

    let path = if thumb_path.exists() {
        thumb_path
    } else if viewer_path.exists() {
        viewer_path
    } else {
        return Err(format!(
            "No cached image for {id} — scroll to it in the filmstrip first"
        ));
    };

    let img = image::open(&path).map_err(|e| format!("Failed to open cached image: {e}"))?;
    Ok(analysis::compute_analysis(&img))
}

// ── Filter ────────────────────────────────────────────────────────────────────

/// Update the active filter and return the new set of visible images.
#[tauri::command]
pub fn set_filter(
    state: State<'_, AppState>,
    filter: FilterPayload,
) -> Result<Vec<ImageInfoDto>, String> {
    let new_filter: SessionFilter = filter.into();
    *state.filter.lock().unwrap() = new_filter.clone();

    let mut session = state.session.lock().unwrap();
    let Some(session) = session.as_mut() else {
        return Ok(vec![]);
    };
    session.clamp_to_visible(&new_filter);

    let visible = session.visible_ids(&new_filter);
    Ok(visible
        .iter()
        .map(|id| {
            let idx = session
                .image_ids
                .iter()
                .position(|&x| x == *id)
                .unwrap_or(0);
            image_dto(&session.images[id], idx)
        })
        .collect())
}
