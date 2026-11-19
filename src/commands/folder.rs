// src/commands/folder.rs
// Folder import (native picker + sidebar tree) and filesystem browsing for
// the folder sidebar (directory listing, Quick Look preview, root locations).

use super::session_dto;
use crate::state::AppState;
use serde::Serialize;
use tauri::State;

use super::SessionInfoDto;

// ── Session commands ──────────────────────────────────────────────────────────

/// Scan `dir` for images, resume any saved decisions, and make it the active
/// session. Shared by `import_folder` (native picker) and `open_folder_at`
/// (sidebar tree) so both paths behave identically.
fn open_session_at(
    dir: std::path::PathBuf,
    state: &State<'_, AppState>,
) -> Result<SessionInfoDto, String> {
    let mut session = crate::core::import::session_from_folder(dir.clone());

    if session.image_ids.is_empty() {
        return Err(format!("No supported images found in {}", dir.display()));
    }

    // Resume saved decisions from the project database.
    let db = match crate::db::Db::open(&dir) {
        Ok(db) => {
            if let Err(e) = db.sync_session(&mut session) {
                tracing::warn!("DB sync failed: {e}");
            }
            Some(db)
        }
        Err(e) => {
            tracing::warn!("Could not open project DB: {e}");
            None
        }
    };

    // Run burst grouping.
    crate::core::burst::assign_groups(&mut session);

    let dto = session_dto(&session);

    *state.session.lock().unwrap() = Some(session);
    *state.db.lock().unwrap() = db;
    state
        .settings
        .lock()
        .unwrap()
        .record_recent_folder(&dir.display().to_string());

    Ok(dto)
}

/// Open a native folder picker, scan for images, resume any saved decisions.
/// Returns Err("cancelled") if the user dismisses the dialog.
#[tauri::command]
pub async fn import_folder(state: State<'_, AppState>) -> Result<SessionInfoDto, String> {
    // Run the blocking folder picker on a dedicated thread.
    let dir = tokio::task::spawn_blocking(|| rfd::FileDialog::new().pick_folder())
        .await
        .map_err(|e| e.to_string())?;

    let Some(dir) = dir else {
        return Err("cancelled".into());
    };

    open_session_at(dir, &state)
}

/// Open a folder at a known path (from the folder sidebar), bypassing the
/// native picker dialog.
#[tauri::command]
pub fn open_folder_at(state: State<'_, AppState>, path: String) -> Result<SessionInfoDto, String> {
    let dir = std::path::PathBuf::from(&path);
    if !dir.is_dir() {
        return Err(format!("Not a folder: {path}"));
    }
    open_session_at(dir, &state)
}

// ── Filesystem browsing (folder sidebar) ────────────────────────────────────────

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DirEntryDto {
    pub name: String,
    pub path: String,
    /// `"folder"` or `"image"`.
    pub kind: String,
}

/// List immediate subdirectories and supported image files of `path`,
/// folders first then alphabetically within each group. Hidden (dot-prefixed)
/// entries and unsupported file types are skipped.
#[tauri::command]
pub fn list_dir_entries(path: String) -> Result<Vec<DirEntryDto>, String> {
    let read_dir = std::fs::read_dir(&path).map_err(|e| e.to_string())?;

    let mut entries: Vec<DirEntryDto> = read_dir
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            if name.starts_with('.') {
                return None;
            }
            let p = e.path();
            if p.is_dir() {
                return Some(DirEntryDto {
                    name,
                    path: p.display().to_string(),
                    kind: "folder".into(),
                });
            }
            let ext = p.extension().and_then(|e| e.to_str())?;
            crate::core::image::ImageFormat::from_extension(ext)?;
            Some(DirEntryDto {
                name,
                path: p.display().to_string(),
                kind: "image".into(),
            })
        })
        .collect();

    entries.sort_by(|a, b| {
        (a.kind != "folder", a.name.to_lowercase())
            .cmp(&(b.kind != "folder", b.name.to_lowercase()))
    });
    Ok(entries)
}

/// Decode a single arbitrary file (not necessarily part of any open session)
/// to a `data:` URL, for the folder sidebar's Quick Look preview.
#[tauri::command]
pub fn preview_file(path: String) -> Result<String, String> {
    use base64::Engine as _;

    let p = std::path::PathBuf::from(&path);
    let ext = p
        .extension()
        .and_then(|e| e.to_str())
        .ok_or_else(|| "No file extension".to_string())?;
    let format = crate::core::image::ImageFormat::from_extension(ext)
        .ok_or_else(|| format!("Unsupported file type: {ext}"))?;

    if format.is_raw() {
        let req = crate::core::thumbnailer::ThumbRequest {
            id: crate::core::image::ImageId::new(),
            path: p,
            format,
            high_res: true,
        };
        let (img, _) = crate::core::thumbnailer::decode_thumbnail_with_exif(&req)
            .map_err(|e| e.to_string())?;

        use image::ImageEncoder as _;
        let rgb = img.to_rgb8();
        let mut jpeg_bytes: Vec<u8> = Vec::new();
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg_bytes, 88)
            .write_image(
                rgb.as_raw(),
                rgb.width(),
                rgb.height(),
                image::ExtendedColorType::Rgb8,
            )
            .map_err(|e| e.to_string())?;
        let b64 = base64::engine::general_purpose::STANDARD.encode(&jpeg_bytes);
        Ok(format!("data:image/jpeg;base64,{b64}"))
    } else {
        let bytes = std::fs::read(&p).map_err(|e| format!("read error: {e}"))?;
        let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
        let mime = match format {
            crate::core::image::ImageFormat::Jpeg => "image/jpeg",
            crate::core::image::ImageFormat::Tiff => "image/tiff",
            _ => "image/jpeg",
        };
        Ok(format!("data:{mime};base64,{b64}"))
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RootLocationDto {
    pub label: String,
    pub path: String,
}

/// Well-known folder-tree roots (Home / Desktop / Pictures) for the sidebar.
#[tauri::command]
pub fn get_root_locations() -> Vec<RootLocationDto> {
    let candidates = [
        ("Home", dirs::home_dir()),
        ("Desktop", dirs::desktop_dir()),
        ("Pictures", dirs::picture_dir()),
    ];

    candidates
        .into_iter()
        .filter_map(|(label, path)| {
            path.map(|p| RootLocationDto {
                label: label.to_string(),
                path: p.display().to_string(),
            })
        })
        .collect()
}
