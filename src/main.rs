// src/main.rs
// Tauri entry point — initialises logging, wires up IPC commands, and launches
// the webview shell.  All image processing and domain logic live in the library
// crate (core/, ai/, db/).

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use anyhow::Result;
use crossbeam::channel::Receiver;
use image::RgbImage;
use serde_json::json;
use shotlist::{
    commands,
    core::{
        session::SessionFilter,
        thumbnailer::{ThumbResult, Thumbnailer},
    },
    settings::AppSettings,
    state::AppState,
};
use std::{path::PathBuf, sync::Mutex};
use tauri::{AppHandle, Emitter, Manager};
use tracing::info;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

fn main() -> Result<()> {
    // ── Logging ────────────────────────────────────────────────────────────
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env().add_directive("shotlist=info".parse()?))
        .init();

    info!("ShotList starting");

    // ── Cache directories ──────────────────────────────────────────────────
    let cache_base = dirs::cache_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("shotlist");
    let thumb_cache_dir = cache_base.join("thumbs");
    let viewer_cache_dir = cache_base.join("viewer");
    std::fs::create_dir_all(&thumb_cache_dir)?;
    std::fs::create_dir_all(&viewer_cache_dir)?;

    // ── Thumbnail pipeline ─────────────────────────────────────────────────
    let thumbnailer = Thumbnailer::spawn();
    let thumb_request_tx = thumbnailer.request_tx;
    let result_rx = thumbnailer.result_rx;

    // ── App state ──────────────────────────────────────────────────────────
    let app_state = AppState {
        session: Mutex::new(None),
        db: Mutex::new(None),
        thumb_request_tx,
        settings: Mutex::new(AppSettings::load()),
        filter: Mutex::new(SessionFilter::show_all()),
        thumb_cache_dir: thumb_cache_dir.clone(),
        viewer_cache_dir: viewer_cache_dir.clone(),
    };

    // ── Tauri ──────────────────────────────────────────────────────────────
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .manage(app_state)
        .setup(move |app| {
            // Start the background thread that drains decoded thumbnails and
            // emits events to the frontend.
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                thumbnail_emitter_loop(result_rx, handle, thumb_cache_dir, viewer_cache_dir);
            });

            // Warm up the face detector in the background.
            std::thread::spawn(shotlist::ai::face_detector::init_detector);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::import_folder,
            commands::open_folder_at,
            commands::list_dir_entries,
            commands::get_root_locations,
            commands::preview_file,
            commands::remove_recent_folder,
            commands::get_session,
            commands::get_visible_images,
            commands::get_current_image,
            commands::navigate,
            commands::jump_to_image,
            commands::get_image_src,
            commands::set_cull_state,
            commands::set_rating,
            commands::request_thumbnails,
            commands::get_zine_thumb_urls,
            commands::export_picks,
            commands::run_auto_cull,
            commands::get_image_analysis,
            commands::set_filter,
            commands::get_settings,
            commands::save_settings,
            commands::export_developed,
            commands::export_all_picks,
            commands::get_caption,
            commands::save_caption,
            commands::generate_caption,
            commands::generate_all_captions,
            commands::embed_captions,
            commands::get_develop_params,
            commands::save_develop_params,
            commands::get_develop_preview,
            commands::auto_develop,
            commands::create_or_get_zine,
            commands::get_zine,
            commands::add_zine_page,
            commands::remove_zine_page,
            commands::set_page_image,
            commands::update_page_order,
            commands::set_page_template,
            commands::set_page_caption_override,
            commands::set_zine_paper_size,
            commands::export_pdf,
            commands::ai_sequence,
            commands::test_lmstudio_connection,
            commands::open_with_app,
        ])
        .run(tauri::generate_context!())
        .expect("ShotList failed to start");

    Ok(())
}

// ── Thumbnail emitter background thread ───────────────────────────────────────

fn thumbnail_emitter_loop(
    result_rx: Receiver<ThumbResult>,
    handle: AppHandle,
    thumb_dir: PathBuf,
    viewer_dir: PathBuf,
) {
    info!("thumbnail_emitter_loop started");
    let mut total = 0u64;
    loop {
        let mut any = false;
        while let Ok(result) = result_rx.try_recv() {
            any = true;
            total += 1;
            tracing::debug!(
                "emitter: got result #{total} id={} hi_res={}",
                result.id,
                result.high_res
            );
            if let Err(e) = emit_thumbnail(&result, &handle, &thumb_dir, &viewer_dir) {
                tracing::warn!("Thumbnail emit error (id={}): {e}", result.id);
            }
        }
        if !any {
            std::thread::sleep(std::time::Duration::from_millis(16));
        }
    }
}

fn emit_thumbnail(
    result: &ThumbResult,
    handle: &AppHandle,
    thumb_dir: &PathBuf,
    viewer_dir: &PathBuf,
) -> anyhow::Result<()> {
    use base64::Engine as _;
    use image::ImageEncoder;

    let img = RgbImage::from_raw(result.width, result.height, result.pixels.clone())
        .ok_or_else(|| anyhow::anyhow!("bad pixel buffer size"))?;

    // Encode JPEG in memory.
    let quality = if result.high_res { 92u8 } else { 82u8 };
    let mut jpeg_bytes: Vec<u8> = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg_bytes, quality).write_image(
        img.as_raw(),
        img.width(),
        img.height(),
        image::ExtendedColorType::Rgb8,
    )?;

    // Also cache to disk (used for get_image_src / viewer cache hits on re-open).
    let cache_dir = if result.high_res {
        viewer_dir
    } else {
        thumb_dir
    };
    let filename = if result.high_res {
        format!("{}_hd.jpg", result.id)
    } else {
        format!("{}.jpg", result.id)
    };
    let _ = std::fs::write(cache_dir.join(&filename), &jpeg_bytes); // best-effort
    let jpeg_len = jpeg_bytes.len();

    // If a geometry-adjusted "developed" variant already exists for this
    // image, prefer it for the emitted event — the pristine file above still
    // gets refreshed as Develop's own editing source, but views elsewhere
    // should reflect the saved rotation/crop.
    let dev_filename = if result.high_res {
        format!("{}_hd_dev.jpg", result.id)
    } else {
        format!("{}_dev.jpg", result.id)
    };
    let emitted_bytes = std::fs::read(cache_dir.join(&dev_filename)).unwrap_or(jpeg_bytes);

    // Emit as a data: URL so the frontend never needs the asset protocol.
    let b64 = base64::engine::general_purpose::STANDARD.encode(&emitted_bytes);
    let data_url = format!("data:image/jpeg;base64,{b64}");

    let _ = handle.emit(
        "thumbnail-ready",
        json!({
            "id":         result.id.to_string(),
            "url":        data_url,
            "highRes":    result.high_res,
            "focusScore": result.focus_score,
            "faceCount":  result.faces.len(),
        }),
    );

    tracing::debug!(
        "emitted thumbnail id={} hi_res={} jpeg_kb={:.1}",
        result.id,
        result.high_res,
        jpeg_len as f32 / 1024.0
    );

    // ── Write decoded signals back into AppState ─────────────────────────────
    // Filmstrip-quality decodes compute focus score, faces, p_hash, and exposure.
    // Write them into the session so commands like run_auto_cull can read them.
    // Hi-res viewer decodes skip signal computation, so we don't overwrite here.
    if !result.high_res {
        let state = handle.state::<AppState>();
        let mut guard = state.session.lock().unwrap();
        if let Some(session) = guard.as_mut() {
            if let Some(entry) = session.images.get_mut(&result.id) {
                entry.assessment.focus_score = Some(result.focus_score);
                entry.assessment.faces = result.faces.clone();
                entry.p_hash = Some(result.p_hash);
                if entry.exposure.is_none() {
                    entry.exposure = Some(result.exposure.clone());
                }
            }
        }
    }

    Ok(())
}
