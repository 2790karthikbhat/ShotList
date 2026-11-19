// src/commands/develop.rs
// Develop stage: saved params load/save, the geometry-only "developed" cache
// regeneration that keeps every other stage in sync, live preview, and AI
// auto-develop suggestions.

use crate::core::image::ImageId;
use crate::state::AppState;
use tauri::{AppHandle, Emitter, State};

/// Load saved develop params for an image (or return defaults if none saved).
#[tauri::command]
pub fn get_develop_params(
    state: State<'_, AppState>,
    image_id: String,
) -> Result<crate::core::develop::DevelopParams, String> {
    if let Some(db) = state.db.lock().unwrap().as_ref() {
        if let Ok(Some(p)) = crate::db::queries::load_develop_params(&db.conn, &image_id) {
            return Ok(p);
        }
    }
    Ok(crate::core::develop::DevelopParams::default())
}

/// Save develop params for an image, then regenerate (or clear) the
/// geometry-only "developed" cache so every other stage reflects the same
/// rotation/crop as Develop, without touching the pristine decode cache
/// Develop's own pipeline depends on as its editing source.
#[tauri::command]
pub fn save_develop_params(
    state: State<'_, AppState>,
    handle: AppHandle,
    image_id: String,
    params: crate::core::develop::DevelopParams,
) -> Result<(), String> {
    {
        let db_guard = state.db.lock().unwrap();
        let db = db_guard.as_ref().ok_or("no project database open")?;
        crate::db::queries::save_develop_params(&db.conn, &image_id, &params)
            .map_err(|e| e.to_string())?;
    }

    if let Ok(id) = image_id.parse::<ImageId>() {
        let entry = state
            .session
            .lock()
            .unwrap()
            .as_ref()
            .and_then(|s| s.images.get(&id).cloned());
        if let Some(entry) = entry {
            if let Err(e) = regenerate_developed_cache(&state, &handle, &image_id, &entry, &params)
            {
                tracing::warn!("Developed-cache regenerate failed for {image_id}: {e}");
            }
        }
    }

    Ok(())
}

/// Regenerate (or clear, if geometry was reset to identity) the developed
/// cache for `entry`, and push a `thumbnail-ready` event with the result so
/// any already-open filmstrip/viewer/zine thumbnail updates instantly
/// instead of waiting for a re-navigate.
fn regenerate_developed_cache(
    state: &State<'_, AppState>,
    handle: &AppHandle,
    image_id: &str,
    entry: &crate::core::image::ImageEntry,
    params: &crate::core::develop::DevelopParams,
) -> anyhow::Result<()> {
    let dev_thumb = state.thumb_cache_dir.join(format!("{image_id}_dev.jpg"));
    let dev_viewer = state
        .viewer_cache_dir
        .join(format!("{image_id}_hd_dev.jpg"));
    let pristine_thumb = state.thumb_cache_dir.join(format!("{image_id}.jpg"));
    let pristine_viewer = state.viewer_cache_dir.join(format!("{image_id}_hd.jpg"));

    if params.rotation == 0 && params.crop.is_none() {
        let _ = std::fs::remove_file(&dev_thumb);
        let _ = std::fs::remove_file(&dev_viewer);
        // Push the pristine bytes back through the same event so open views
        // revert immediately rather than waiting for a re-navigate.
        if pristine_thumb.exists() {
            push_thumbnail_ready(handle, image_id, &pristine_thumb, false)?;
        }
        if pristine_viewer.exists() {
            push_thumbnail_ready(handle, image_id, &pristine_viewer, true)?;
        }
        return Ok(());
    }

    // Source: decode fresh from the original file rather than depending on
    // the pristine HD viewer cache — that cache is populated asynchronously
    // by the background thumbnailer and can still be missing right after a
    // quick edit-then-save, which silently skipped regeneration entirely.
    let img = if entry.format.is_raw() {
        let req = crate::core::thumbnailer::ThumbRequest {
            id: entry.id,
            path: entry.path.clone(),
            format: entry.format,
            high_res: true,
        };
        crate::core::thumbnailer::decode_thumbnail_with_exif(&req)?.0
    } else {
        image::open(&entry.path)?
    };

    let geo = crate::core::develop::apply_geometry(&img, params.rotation, params.crop);

    write_jpeg_file(&geo, &dev_viewer, 92)?;
    let thumb = crate::core::thumbnailer::fit_to_size(geo, crate::core::thumbnailer::THUMB_SIZE);
    write_jpeg_file(&thumb, &dev_thumb, 82)?;

    push_thumbnail_ready(handle, image_id, &dev_thumb, false)?;
    push_thumbnail_ready(handle, image_id, &dev_viewer, true)?;

    Ok(())
}

fn write_jpeg_file(
    img: &image::DynamicImage,
    path: &std::path::Path,
    quality: u8,
) -> anyhow::Result<()> {
    use image::ImageEncoder as _;
    let rgb = img.to_rgb8();
    let mut bytes: Vec<u8> = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut bytes, quality).write_image(
        rgb.as_raw(),
        rgb.width(),
        rgb.height(),
        image::ExtendedColorType::Rgb8,
    )?;
    std::fs::write(path, &bytes)?;
    Ok(())
}

fn push_thumbnail_ready(
    handle: &AppHandle,
    image_id: &str,
    path: &std::path::Path,
    high_res: bool,
) -> anyhow::Result<()> {
    use base64::Engine as _;
    let bytes = std::fs::read(path)?;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
    let data_url = format!("data:image/jpeg;base64,{b64}");
    let _ = handle.emit(
        "thumbnail-ready",
        serde_json::json!({
            "id":         image_id,
            "url":        data_url,
            "highRes":    high_res,
            "focusScore": 0.0,
            "faceCount":  0,
        }),
    );
    Ok(())
}

/// Apply develop params to the cached thumbnail and return a data: URL.
/// Used for the live preview panel in the Develop stage.
#[tauri::command]
pub fn get_develop_preview(
    state: State<'_, AppState>,
    image_id: String,
    params: crate::core::develop::DevelopParams,
) -> Result<String, String> {
    use crate::core::develop::apply_develop;
    use base64::Engine as _;

    // Prefer full viewer-quality image for preview; fall back to filmstrip thumb.
    let viewer_path = state.viewer_cache_dir.join(format!("{image_id}_hd.jpg"));
    let thumb_path = state.thumb_cache_dir.join(format!("{image_id}.jpg"));

    let path = if viewer_path.exists() {
        viewer_path
    } else if thumb_path.exists() {
        thumb_path
    } else {
        return Err(format!(
            "No cached image for {image_id} — scroll to it in the filmstrip first"
        ));
    };

    let img = image::open(&path).map_err(|e| format!("Failed to open image: {e}"))?;
    let processed = apply_develop(&img, &params);

    // Encode processed image as JPEG.
    let rgb = processed.to_rgb8();
    let mut jpeg_bytes: Vec<u8> = Vec::new();
    use image::ImageEncoder as _;
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg_bytes, 88)
        .write_image(
            rgb.as_raw(),
            rgb.width(),
            rgb.height(),
            image::ExtendedColorType::Rgb8,
        )
        .map_err(|e| format!("JPEG encode error: {e}"))?;

    let b64 = base64::engine::general_purpose::STANDARD.encode(&jpeg_bytes);
    Ok(format!("data:image/jpeg;base64,{b64}"))
}

/// Stream AI develop suggestions for an image (Anthropic or LM Studio).
/// Suggestions arrive as `develop-suggestion` Tauri events while the command runs.
#[tauri::command]
pub async fn auto_develop(
    state: State<'_, AppState>,
    image_id: String,
    handle: tauri::AppHandle,
) -> Result<(), String> {
    use crate::ai::develop;
    use crate::core::analysis::compute_analysis;

    let (ai_provider, anthropic_key, lmstudio_url, lmstudio_model) = {
        let s = state.settings.lock().unwrap();
        (
            s.ai_provider.clone(),
            s.anthropic_key.clone(),
            s.lmstudio_url.clone(),
            s.lmstudio_model.clone(),
        )
    };

    // Load thumbnail (filmstrip thumb preferred; fall back to HD viewer cache).
    let thumb_path = state.thumb_cache_dir.join(format!("{image_id}.jpg"));
    let viewer_path = state.viewer_cache_dir.join(format!("{image_id}_hd.jpg"));
    let path = if thumb_path.exists() {
        thumb_path
    } else if viewer_path.exists() {
        viewer_path
    } else {
        return Err(format!("No cached image for {image_id}"));
    };

    let img = image::open(&path).map_err(|e| e.to_string())?;
    // Run technical analysis on the full-resolution image for accurate zone/luma data.
    let analysis = compute_analysis(&img);

    // Resize before sending to the AI vision API.
    // compute_analysis already ran on the original; the AI only needs visual content.
    let send_img = {
        let max = 1024u32;
        if img.width() > max || img.height() > max {
            img.resize(max, max, image::imageops::FilterType::Lanczos3)
        } else {
            img
        }
    };

    let rgb = send_img.to_rgb8();
    let mut jpeg: Vec<u8> = Vec::new();
    use image::ImageEncoder as _;
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, 80)
        .write_image(
            rgb.as_raw(),
            rgb.width(),
            rgb.height(),
            image::ExtendedColorType::Rgb8,
        )
        .map_err(|e| e.to_string())?;
    use base64::Engine as _;
    let jpeg_b64 = base64::engine::general_purpose::STANDARD.encode(&jpeg);

    if ai_provider == "lmstudio" {
        if lmstudio_model.is_empty() {
            return Err("No LM Studio model configured — add one in Settings".into());
        }
        return develop::run_lmstudio(
            &lmstudio_url,
            &lmstudio_model,
            &jpeg_b64,
            &analysis,
            &handle,
        )
        .await
        .map_err(|e| e.to_string());
    }

    #[cfg(feature = "provider-anthropic")]
    {
        if anthropic_key.is_empty() {
            return Err("No Anthropic API key set — add it in Settings".into());
        }
        return develop::run(&anthropic_key, &jpeg_b64, &analysis, &handle)
            .await
            .map_err(|e| e.to_string());
    }

    #[cfg(not(feature = "provider-anthropic"))]
    {
        let _ = anthropic_key;
        Ok(())
    }
}
