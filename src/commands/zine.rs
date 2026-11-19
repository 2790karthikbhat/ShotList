// src/commands/zine.rs
// Zine builder: page CRUD, ordering, template/paper-size, PDF export, AI sequencing.

use crate::state::AppState;
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

/// Load (or create) the zine for the current session.
/// On first open, auto-populates with all picks so the user can trim/reorder rather than start blank.
#[tauri::command]
pub fn create_or_get_zine(state: State<'_, AppState>) -> Result<crate::core::zine::Zine, String> {
    use crate::core::image::CullState;
    use crate::db::queries as q;

    let session_guard = state.session.lock().unwrap();
    let session = session_guard.as_ref().ok_or("no session")?;
    let session_id = session.id.to_string();

    let db_guard = state.db.lock().unwrap();
    let db = db_guard.as_ref().ok_or("no project database open")?;

    let mut zine = q::create_or_get_zine(&db.conn, &session_id).map_err(|e| e.to_string())?;

    // Seed from picks when: (a) brand new zine with no pages, or (b) every existing
    // page has no image assigned — which happens when old test/empty pages were
    // persisted before this feature existed.
    let needs_seed = zine.pages.is_empty() || zine.pages.iter().all(|p| p.image_id.is_none());

    if needs_seed {
        // Wipe any imageless placeholder pages left over from earlier sessions.
        for page in &zine.pages {
            q::remove_zine_page(&db.conn, &page.id).map_err(|e| e.to_string())?;
        }
        zine.pages.clear();

        let picks: Vec<_> = session
            .image_ids
            .iter()
            .filter_map(|id| session.images.get(id))
            .filter(|e| e.cull_state == CullState::Pick)
            .collect();

        for (i, entry) in picks.iter().enumerate() {
            let page = q::add_zine_page(
                &db.conn,
                &zine.id,
                i as i64,
                Some(&entry.id.to_string()),
                &crate::core::zine::TemplateId::Top75,
            )
            .map_err(|e| e.to_string())?;
            zine.pages.push(page);
        }
    }

    Ok(zine)
}

/// Reload the current zine (after external changes such as ai_sequence).
#[tauri::command]
pub fn get_zine(state: State<'_, AppState>) -> Result<crate::core::zine::Zine, String> {
    use crate::db::queries as q;

    let session_guard = state.session.lock().unwrap();
    let session = session_guard.as_ref().ok_or("no session")?;
    let session_id = session.id.to_string();
    drop(session_guard);

    let db_guard = state.db.lock().unwrap();
    let db = db_guard.as_ref().ok_or("no project database open")?;

    q::create_or_get_zine(&db.conn, &session_id).map_err(|e| e.to_string())
}

/// Reorder pages by supplying their IDs in the new desired order.
#[tauri::command]
pub fn update_page_order(state: State<'_, AppState>, page_ids: Vec<String>) -> Result<(), String> {
    let db_guard = state.db.lock().unwrap();
    let db = db_guard.as_ref().ok_or("no project database open")?;
    crate::db::queries::update_page_order(&db.conn, &page_ids).map_err(|e| e.to_string())
}

/// Change the layout template for a single zine page.
#[tauri::command]
pub fn set_page_template(
    state: State<'_, AppState>,
    page_id: String,
    template: String,
) -> Result<(), String> {
    let template = crate::core::zine::TemplateId::from_label(&template);
    let db_guard = state.db.lock().unwrap();
    let db = db_guard.as_ref().ok_or("no project database open")?;
    crate::db::queries::set_page_template(&db.conn, &page_id, &template).map_err(|e| e.to_string())
}

/// Set (or clear) the caption override for a page.
#[tauri::command]
pub fn set_page_caption_override(
    state: State<'_, AppState>,
    page_id: String,
    override_: Option<String>,
) -> Result<(), String> {
    let db_guard = state.db.lock().unwrap();
    let db = db_guard.as_ref().ok_or("no project database open")?;
    crate::db::queries::set_page_caption_override(&db.conn, &page_id, override_.as_deref())
        .map_err(|e| e.to_string())
}

/// Remove a page from the zine entirely.
#[tauri::command]
pub fn remove_zine_page(state: State<'_, AppState>, page_id: String) -> Result<(), String> {
    let db_guard = state.db.lock().unwrap();
    let db = db_guard.as_ref().ok_or("no project database open")?;
    crate::db::queries::remove_zine_page(&db.conn, &page_id).map_err(|e| e.to_string())
}

/// Set (or clear) which image is displayed on a page.
#[tauri::command]
pub fn set_page_image(
    state: State<'_, AppState>,
    page_id: String,
    image_id: Option<String>,
    #[allow(unused_variables)] slot: Option<u8>,
) -> Result<(), String> {
    let db_guard = state.db.lock().unwrap();
    let db = db_guard.as_ref().ok_or("no project database open")?;
    crate::db::queries::set_page_image_slot(
        &db.conn,
        &page_id,
        slot.unwrap_or(1),
        image_id.as_deref(),
    )
    .map_err(|e| e.to_string())
}

/// Add a new page to the zine (with an optional image).
#[tauri::command]
pub fn add_zine_page(
    state: State<'_, AppState>,
    zine_id: String,
    image_id: Option<String>,
    template: String,
) -> Result<crate::core::zine::ZinePage, String> {
    use crate::db::queries as q;

    let db_guard = state.db.lock().unwrap();
    let db = db_guard.as_ref().ok_or("no project database open")?;

    // Determine the next position
    let pages = q::load_zine_pages(&db.conn, &zine_id).map_err(|e| e.to_string())?;
    let position = pages.iter().map(|p| p.position).max().unwrap_or(-1) + 1;
    let template = crate::core::zine::TemplateId::from_label(&template);

    q::add_zine_page(&db.conn, &zine_id, position, image_id.as_deref(), &template)
        .map_err(|e| e.to_string())
}

/// Update the paper size for the zine.
#[tauri::command]
pub fn set_zine_paper_size(
    state: State<'_, AppState>,
    zine_id: String,
    paper_size: String,
) -> Result<(), String> {
    let size = crate::core::zine::PaperSize::from_label(&paper_size);
    let db_guard = state.db.lock().unwrap();
    let db = db_guard.as_ref().ok_or("no project database open")?;
    crate::db::queries::set_zine_paper_size(&db.conn, &zine_id, &size).map_err(|e| e.to_string())
}

/// Export progress event for PDF export.
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PdfExportProgress {
    pub page: usize,
    pub total: usize,
    pub message: String,
}

/// Render the zine to a PDF and save to <source_dir>/Zine/zine.pdf.
/// Returns the output path on success.
#[tauri::command]
pub async fn export_pdf(
    state: State<'_, AppState>,
    quality: u8,
    handle: AppHandle,
) -> Result<String, String> {
    use crate::core::develop::apply_develop;
    use printpdf::RawImage;
    use std::collections::HashMap;

    let (zine, source_dir, viewer_dir, thumb_dir) = {
        let session_guard = state.session.lock().unwrap();
        let session = session_guard.as_ref().ok_or("no session")?;
        let session_id = session.id.to_string();

        let db_guard = state.db.lock().unwrap();
        let db = db_guard.as_ref().ok_or("no project database open")?;

        let zine = crate::db::queries::create_or_get_zine(&db.conn, &session_id)
            .map_err(|e| e.to_string())?;

        (
            zine,
            session.source_dir.clone(),
            state.viewer_cache_dir.clone(),
            state.thumb_cache_dir.clone(),
        )
    };

    let total = zine.pages.len();
    if total == 0 {
        return Err("Zine has no pages. Add some picks first.".into());
    }

    let _ = handle.emit(
        "pdf-export-progress",
        PdfExportProgress {
            page: 0,
            total,
            message: "Loading images…".into(),
        },
    );

    // Load captions from DB for all pages.
    let captions = {
        let db_guard = state.db.lock().unwrap();
        let db = db_guard.as_ref().ok_or("no project database open")?;
        let image_ids: Vec<String> = zine
            .pages
            .iter()
            .filter_map(|p| p.image_id.clone())
            .collect();
        crate::db::queries::load_all_captions(&db.conn, &image_ids).map_err(|e| e.to_string())?
    };

    // Load and develop images for PDF embedding.
    let mut images: HashMap<String, RawImage> = HashMap::new();
    let q = quality.clamp(50, 100);

    // Collect every unique image ID across all slots.
    let all_image_ids: Vec<String> = {
        let mut ids = std::collections::HashSet::new();
        for p in &zine.pages {
            for id in [
                &p.image_id,
                &p.image_id_2,
                &p.image_id_3,
                &p.image_id_4,
                &p.image_id_5,
                &p.image_id_6,
                &p.image_id_7,
                &p.image_id_8,
                &p.image_id_9,
            ]
            .into_iter()
            .flatten()
            {
                ids.insert(id.clone());
            }
        }
        ids.into_iter().collect()
    };

    for (i, image_id) in all_image_ids.iter().enumerate() {
        let _ = handle.emit(
            "pdf-export-progress",
            PdfExportProgress {
                page: i,
                total,
                message: format!("Processing image {}…", i + 1),
            },
        );

        // Load image bytes.
        let hd = viewer_dir.join(format!("{image_id}_hd.jpg"));
        let thumb = thumb_dir.join(format!("{image_id}.jpg"));
        let path = if hd.exists() {
            hd
        } else if thumb.exists() {
            thumb
        } else {
            continue;
        };

        let img = match image::open(&path) {
            Ok(i) => i,
            Err(e) => {
                tracing::warn!("PDF: skip {image_id}: {e}");
                continue;
            }
        };

        // Apply develop params if saved.
        let params = {
            let db_guard = state.db.lock().unwrap();
            db_guard
                .as_ref()
                .and_then(|db| {
                    crate::db::queries::load_develop_params(&db.conn, image_id)
                        .ok()
                        .flatten()
                })
                .unwrap_or_default()
        };
        let processed = apply_develop(&img, &params);

        // Encode to JPEG then decode into RawImage for printpdf.
        let rgb = processed.to_rgb8();
        let mut jpeg_bytes: Vec<u8> = Vec::new();
        use image::ImageEncoder as _;
        if image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg_bytes, q)
            .write_image(
                rgb.as_raw(),
                rgb.width(),
                rgb.height(),
                image::ExtendedColorType::Rgb8,
            )
            .is_err()
        {
            continue;
        }

        let mut warnings = Vec::new();
        if let Ok(raw) = RawImage::decode_from_bytes(&jpeg_bytes, &mut warnings) {
            images.insert(image_id.clone(), raw);
        }
    }

    let _ = handle.emit(
        "pdf-export-progress",
        PdfExportProgress {
            page: total,
            total,
            message: "Rendering PDF…".into(),
        },
    );

    let pdf_bytes =
        crate::core::pdf::render_pdf(&zine, &images, &captions).map_err(|e| e.to_string())?;

    let out_dir = source_dir.join("Zine");
    std::fs::create_dir_all(&out_dir).map_err(|e| e.to_string())?;
    let out_path = out_dir.join("zine.pdf");
    std::fs::write(&out_path, &pdf_bytes).map_err(|e| e.to_string())?;

    Ok(format!(
        "PDF exported → {} ({} pages)",
        out_path.display(),
        total
    ))
}

/// Stream an AI narrative sequence suggestion for the zine (Anthropic or LM Studio).
/// Suggestions arrive as `sequence-suggestion` Tauri events.
/// Applies the suggested order to the DB automatically.
#[tauri::command]
pub async fn ai_sequence(state: State<'_, AppState>, handle: AppHandle) -> Result<String, String> {
    use crate::ai::sequence;
    use crate::db::queries as q;

    let (ai_provider, anthropic_key, lmstudio_url, lmstudio_model) = {
        let s = state.settings.lock().unwrap();
        (
            s.ai_provider.clone(),
            s.anthropic_key.clone(),
            s.lmstudio_url.clone(),
            s.lmstudio_model.clone(),
        )
    };

    let (zine, captions) = {
        let session_guard = state.session.lock().unwrap();
        let session = session_guard.as_ref().ok_or("no session")?;
        let session_id = session.id.to_string();

        let db_guard = state.db.lock().unwrap();
        let db = db_guard.as_ref().ok_or("no project database open")?;

        let zine = q::create_or_get_zine(&db.conn, &session_id).map_err(|e| e.to_string())?;

        let image_ids: Vec<String> = zine
            .pages
            .iter()
            .filter_map(|p| p.image_id.clone())
            .collect();
        let captions = q::load_all_captions(&db.conn, &image_ids).map_err(|e| e.to_string())?;

        (zine, captions)
    };

    if zine.pages.is_empty() {
        return Err("Zine has no pages. Add some picks first.".into());
    }

    let image_ids_in_order = if ai_provider == "lmstudio" {
        if lmstudio_model.is_empty() {
            return Err("No LM Studio model configured — add one in Settings".into());
        }
        sequence::run_lmstudio(
            &lmstudio_url,
            &lmstudio_model,
            &zine.pages,
            &captions,
            &handle,
        )
        .await
        .map_err(|e| e.to_string())?
    } else {
        #[cfg(feature = "provider-anthropic")]
        {
            if anthropic_key.is_empty() {
                return Err("No Anthropic API key set — add it in Settings".into());
            }
            sequence::run(&anthropic_key, &zine.pages, &captions, &handle)
                .await
                .map_err(|e| e.to_string())?
        }
        #[cfg(not(feature = "provider-anthropic"))]
        {
            let _ = anthropic_key;
            return Err("AI features require the provider-anthropic feature".to_string());
        }
    };

    let id_to_page: std::collections::HashMap<_, _> = zine
        .pages
        .iter()
        .filter_map(|p| {
            p.image_id
                .as_ref()
                .map(|img_id| (img_id.as_str(), p.id.as_str()))
        })
        .collect();

    tracing::info!(
        "ai_sequence: parsed {} image_ids from model: {:?}",
        image_ids_in_order.len(),
        image_ids_in_order
    );
    tracing::info!(
        "ai_sequence: known image_ids in zine: {:?}",
        id_to_page.keys().collect::<Vec<_>>()
    );

    let new_order: Vec<String> = image_ids_in_order
        .iter()
        .filter(|id| !id.is_empty())
        .filter_map(|img_id| id_to_page.get(img_id.as_str()).map(|s| s.to_string()))
        .collect();

    tracing::info!(
        "ai_sequence: new_order has {} pages (out of {})",
        new_order.len(),
        zine.pages.len()
    );

    if !new_order.is_empty() {
        let db_guard = state.db.lock().unwrap();
        let db = db_guard.as_ref().ok_or("no project database open")?;
        q::update_page_order(&db.conn, &new_order).map_err(|e| e.to_string())?;
    }

    let provider_name = if ai_provider == "lmstudio" {
        "LM Studio"
    } else {
        "Claude"
    };
    let msg = if new_order.is_empty() {
        "AI sequencing: could not match model output to zine pages — check terminal logs"
            .to_string()
    } else {
        format!(
            "Sequence updated — {} of {} pages reordered by {}",
            new_order.len(),
            zine.pages.len(),
            provider_name
        )
    };
    Ok(msg)
}
