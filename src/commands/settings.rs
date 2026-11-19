// src/commands/settings.rs
// App settings persistence and the folder sidebar's recent-folders list.

use crate::settings::AppSettings;
use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Result<AppSettings, String> {
    Ok(state.settings.lock().unwrap().clone())
}

#[tauri::command]
pub fn save_settings(state: State<'_, AppState>, settings: AppSettings) -> Result<(), String> {
    settings.save();
    *state.settings.lock().unwrap() = settings;
    Ok(())
}

#[tauri::command]
pub fn remove_recent_folder(state: State<'_, AppState>, path: String) -> Result<(), String> {
    state.settings.lock().unwrap().remove_recent_folder(&path);
    Ok(())
}
