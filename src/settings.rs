// src/settings.rs
// Persistent application preferences.
// Previously lived in src/ui/app_settings.rs — lifted here so it is
// accessible without the egui ui module.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// rename_all = "camelCase" makes Tauri IPC round-trips work correctly
// (the frontend sends camelCase JSON; serde needs matching names).
// alias attributes on each field let old snake_case settings.json files
// continue to load without data loss.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    #[serde(default, alias = "anthropic_key")]
    pub anthropic_key: String,
    #[serde(default, alias = "openai_key")]
    pub openai_key: String,
    #[serde(default = "default_true", alias = "auto_advance")]
    pub auto_advance: bool,
    #[serde(default = "default_true", alias = "show_rejects")]
    pub show_rejects: bool,
    #[serde(default = "default_ai_provider", alias = "ai_provider")]
    pub ai_provider: String,
    #[serde(default = "default_lmstudio_url", alias = "lmstudio_url")]
    pub lmstudio_url: String,
    #[serde(default, alias = "lmstudio_model")]
    pub lmstudio_model: String,
    #[serde(default, alias = "recent_folders")]
    pub recent_folders: Vec<String>,
    /// "dark" | "light" | "system".
    #[serde(default = "default_theme")]
    pub theme: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            anthropic_key: String::new(),
            openai_key: String::new(),
            auto_advance: true,
            show_rejects: true,
            ai_provider: default_ai_provider(),
            lmstudio_url: default_lmstudio_url(),
            lmstudio_model: String::new(),
            recent_folders: Vec::new(),
            theme: default_theme(),
        }
    }
}

fn default_true() -> bool {
    true
}
fn default_ai_provider() -> String {
    "anthropic".to_string()
}
fn default_lmstudio_url() -> String {
    "http://localhost:1234/v1".to_string()
}
fn default_theme() -> String {
    "dark".to_string()
}

impl AppSettings {
    pub fn load() -> Self {
        let path = Self::path();
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(&path, json);
        }
    }

    fn path() -> PathBuf {
        dirs::config_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("shotlist")
            .join("settings.json")
    }

    /// Record a newly-opened folder at the front of `recent_folders`,
    /// deduplicating and capping the list, then persist to disk.
    pub fn record_recent_folder(&mut self, path: &str) {
        self.recent_folders.retain(|p| p != path);
        self.recent_folders.insert(0, path.to_string());
        self.recent_folders.truncate(8);
        self.save();
    }

    /// Remove a folder from `recent_folders` (user-initiated), then persist.
    pub fn remove_recent_folder(&mut self, path: &str) {
        self.recent_folders.retain(|p| p != path);
        self.save();
    }
}
