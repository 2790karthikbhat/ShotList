// src/commands/system.rs
// External-system integration: LM Studio connectivity probe, "open with app".

/// Probe LM Studio's /models endpoint. Returns list of loaded model IDs on success.
#[tauri::command]
pub async fn test_lmstudio_connection(base_url: String) -> Result<Vec<String>, String> {
    crate::ai::lmstudio::fetch_models(&base_url)
        .await
        .map_err(|e| e.to_string())
}

// ── Open with external app ─────────────────────────────────────────────────────

#[tauri::command]
pub async fn open_with_app(image_path: String, app_name: Option<String>) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let mut cmd = std::process::Command::new("open");
        if let Some(ref app) = app_name {
            cmd.arg("-a").arg(app);
        }
        cmd.arg(&image_path)
            .spawn()
            .map_err(|e| format!("Failed to open: {e}"))?;
    }
    #[cfg(target_os = "windows")]
    {
        let _ = app_name;
        std::process::Command::new("cmd")
            .args(["/c", "start", "", &image_path])
            .spawn()
            .map_err(|e| format!("Failed to open: {e}"))?;
    }
    #[cfg(target_os = "linux")]
    {
        let _ = app_name;
        std::process::Command::new("xdg-open")
            .arg(&image_path)
            .spawn()
            .map_err(|e| format!("Failed to open: {e}"))?;
    }
    Ok(())
}
