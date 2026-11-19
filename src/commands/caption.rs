// src/commands/caption.rs
// Caption stage: load/save, single AI generation, batch AI generation, XMP embed.

use crate::state::AppState;
use tauri::{Emitter, State};

/// Load saved caption for an image (returns all-empty default if none saved).
#[tauri::command]
pub fn get_caption(
    state: State<'_, AppState>,
    image_id: String,
) -> Result<crate::core::caption::Caption, String> {
    if let Some(db) = state.db.lock().unwrap().as_ref() {
        if let Ok(Some(c)) = crate::db::queries::load_caption(&db.conn, &image_id) {
            return Ok(c);
        }
    }
    Ok(crate::core::caption::Caption::default())
}

/// Save (user-edited) caption for an image.
#[tauri::command]
pub fn save_caption(
    state: State<'_, AppState>,
    image_id: String,
    caption: crate::core::caption::Caption,
) -> Result<(), String> {
    let db_guard = state.db.lock().unwrap();
    let db = db_guard.as_ref().ok_or("no project database open")?;
    crate::db::queries::save_caption(&db.conn, &image_id, &caption).map_err(|e| e.to_string())
}

/// Stream an AI caption for one image (Anthropic or LM Studio).
/// Fields arrive as `caption-field` Tauri events while the command runs.
/// The final caption is also auto-saved to the DB.
#[tauri::command]
pub async fn generate_caption(
    state: State<'_, AppState>,
    image_id: String,
    handle: tauri::AppHandle,
) -> Result<crate::core::caption::Caption, String> {
    use crate::ai::caption;

    let (ai_provider, anthropic_key, lmstudio_url, lmstudio_model) = {
        let s = state.settings.lock().unwrap();
        (
            s.ai_provider.clone(),
            s.anthropic_key.clone(),
            s.lmstudio_url.clone(),
            s.lmstudio_model.clone(),
        )
    };

    // Load thumbnail (prefer HD viewer, fall back to filmstrip thumb).
    let thumb_path = state.thumb_cache_dir.join(format!("{image_id}.jpg"));
    let viewer_path = state.viewer_cache_dir.join(format!("{image_id}_hd.jpg"));
    let path = if viewer_path.exists() {
        viewer_path
    } else if thumb_path.exists() {
        thumb_path
    } else {
        return Err(format!("No cached image for {image_id}"));
    };

    let img = image::open(&path).map_err(|e| e.to_string())?;

    // Resize to ≤1024px (long edge) to keep API costs reasonable.
    let img = {
        let max = 1024u32;
        if img.width() > max || img.height() > max {
            img.resize(max, max, image::imageops::FilterType::Lanczos3)
        } else {
            img
        }
    };

    let rgb = img.to_rgb8();
    let mut jpeg: Vec<u8> = Vec::new();
    use image::ImageEncoder as _;
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, 82)
        .write_image(
            rgb.as_raw(),
            rgb.width(),
            rgb.height(),
            image::ExtendedColorType::Rgb8,
        )
        .map_err(|e| e.to_string())?;
    use base64::Engine as _;
    let jpeg_b64 = base64::engine::general_purpose::STANDARD.encode(&jpeg);

    let cap = if ai_provider == "lmstudio" {
        if lmstudio_model.is_empty() {
            return Err("No LM Studio model configured — add one in Settings".into());
        }
        caption::run_lmstudio(&lmstudio_url, &lmstudio_model, &jpeg_b64, &handle)
            .await
            .map_err(|e| e.to_string())?
    } else {
        #[cfg(feature = "provider-anthropic")]
        {
            if anthropic_key.is_empty() {
                return Err("No Anthropic API key set — add it in Settings".into());
            }
            caption::run(&anthropic_key, &jpeg_b64, &handle)
                .await
                .map_err(|e| e.to_string())?
        }
        #[cfg(not(feature = "provider-anthropic"))]
        {
            let _ = anthropic_key;
            return Ok(crate::core::caption::Caption::default());
        }
    };

    // Auto-save to DB.
    if let Some(db) = state.db.lock().unwrap().as_ref() {
        let _ = crate::db::queries::save_caption_generated(&db.conn, &image_id, &cap);
    }

    Ok(cap)
}

// ── Batch caption events ──────────────────────────────────────────────────────

#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CaptionBatchProgress {
    pub index: usize,
    pub total: usize,
    pub image_id: String,
}

#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CaptionBatchDone {
    pub image_id: String,
    pub caption: crate::core::caption::Caption,
}

/// Generate captions for the given image IDs (the frontend passes only the
/// currently-filtered visible images, so the active filter is honoured).
/// Emits `caption-batch-progress` before each image and `caption-batch-done`
/// after each succeeds. Already-captioned images are skipped unless
/// `overwrite` is true.
#[tauri::command]
pub async fn generate_all_captions(
    state: State<'_, AppState>,
    handle: tauri::AppHandle,
    image_ids: Vec<String>,
    overwrite: bool,
) -> Result<String, String> {
    use crate::ai::caption;

    if image_ids.is_empty() {
        return Ok("No images to caption".into());
    }

    let (ai_provider, anthropic_key, lmstudio_url, lmstudio_model) = {
        let s = state.settings.lock().unwrap();
        (
            s.ai_provider.clone(),
            s.anthropic_key.clone(),
            s.lmstudio_url.clone(),
            s.lmstudio_model.clone(),
        )
    };

    if ai_provider == "lmstudio" && lmstudio_model.is_empty() {
        return Err("No LM Studio model configured — add one in Settings".into());
    }
    #[cfg(feature = "provider-anthropic")]
    if ai_provider != "lmstudio" && anthropic_key.is_empty() {
        return Err("No Anthropic API key set — add it in Settings".into());
    }

    // Determine which images already have captions.
    let existing: std::collections::HashSet<String> = {
        let db_guard = state.db.lock().unwrap();
        let db = db_guard.as_ref().ok_or("no project database open")?;
        crate::db::queries::load_all_captions(&db.conn, &image_ids)
            .map_err(|e| e.to_string())?
            .into_keys()
            .collect()
    };

    let to_process: Vec<String> = image_ids
        .into_iter()
        .filter(|id| overwrite || !existing.contains(id))
        .collect();

    let total = to_process.len();
    let skipped = if overwrite { 0 } else { existing.len() };
    let mut generated = 0usize;
    let mut errors = 0usize;

    for (index, image_id) in to_process.iter().enumerate() {
        let _ = handle.emit(
            "caption-batch-progress",
            CaptionBatchProgress {
                index,
                total,
                image_id: image_id.clone(),
            },
        );

        // Load + resize image — same as generate_caption.
        let thumb_path = state.thumb_cache_dir.join(format!("{image_id}.jpg"));
        let viewer_path = state.viewer_cache_dir.join(format!("{image_id}_hd.jpg"));
        let path = if viewer_path.exists() {
            viewer_path
        } else if thumb_path.exists() {
            thumb_path
        } else {
            errors += 1;
            continue;
        };

        let img = match image::open(&path) {
            Ok(i) => i,
            Err(_) => {
                errors += 1;
                continue;
            }
        };
        let img = {
            let max = 1024u32;
            if img.width() > max || img.height() > max {
                img.resize(max, max, image::imageops::FilterType::Lanczos3)
            } else {
                img
            }
        };
        let rgb = img.to_rgb8();
        let mut jpeg: Vec<u8> = Vec::new();
        use image::ImageEncoder as _;
        if image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, 82)
            .write_image(
                rgb.as_raw(),
                rgb.width(),
                rgb.height(),
                image::ExtendedColorType::Rgb8,
            )
            .is_err()
        {
            errors += 1;
            continue;
        }
        use base64::Engine as _;
        let jpeg_b64 = base64::engine::general_purpose::STANDARD.encode(&jpeg);

        let result = if ai_provider == "lmstudio" {
            caption::run_lmstudio(&lmstudio_url, &lmstudio_model, &jpeg_b64, &handle).await
        } else {
            #[cfg(feature = "provider-anthropic")]
            {
                caption::run(&anthropic_key, &jpeg_b64, &handle).await
            }
            #[cfg(not(feature = "provider-anthropic"))]
            {
                Ok(crate::core::caption::Caption::default())
            }
        };

        match result {
            Ok(cap) => {
                if let Some(db) = state.db.lock().unwrap().as_ref() {
                    let _ = crate::db::queries::save_caption_generated(&db.conn, image_id, &cap);
                }
                let _ = handle.emit(
                    "caption-batch-done",
                    CaptionBatchDone {
                        image_id: image_id.clone(),
                        caption: cap,
                    },
                );
                generated += 1;
            }
            Err(e) => {
                tracing::warn!("batch caption failed for {image_id}: {e}");
                errors += 1;
            }
        }
    }

    let mut parts = vec![format!("Generated {generated} captions")];
    if skipped > 0 {
        parts.push(format!("{skipped} already had captions (skipped)"));
    }
    if errors > 0 {
        parts.push(format!("{errors} failed"));
    }
    Ok(parts.join(" · "))
}

/// Embed captions for all images that have them into XMP sidecars.
/// Returns a summary string ("Embedded N captions, M skipped").
#[tauri::command]
pub fn embed_captions(state: State<'_, AppState>) -> Result<String, String> {
    let session_guard = state.session.lock().unwrap();
    let session = session_guard.as_ref().ok_or("no session")?;

    let db_guard = state.db.lock().unwrap();
    let db = db_guard.as_ref().ok_or("no project database open")?;

    let image_ids: Vec<String> = session.image_ids.iter().map(|id| id.to_string()).collect();

    let captions =
        crate::db::queries::load_all_captions(&db.conn, &image_ids).map_err(|e| e.to_string())?;

    let mut embedded = 0usize;
    let mut skipped = 0usize;

    for id in &session.image_ids {
        let id_str = id.to_string();
        let entry = &session.images[id];

        if let Some(caption) = captions.get(&id_str) {
            match crate::core::xmp::write_sidecar_with_caption(entry, caption) {
                Ok(_) => embedded += 1,
                Err(e) => {
                    tracing::warn!("XMP embed failed for {:?}: {e}", entry.path);
                    skipped += 1;
                }
            }
        } else {
            skipped += 1;
        }
    }

    Ok(format!(
        "Embedded {embedded} captions into XMP sidecars ({skipped} images skipped)"
    ))
}
