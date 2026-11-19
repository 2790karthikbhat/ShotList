// src/state.rs
// Global Tauri application state — shared across all IPC command handlers.

use crate::core::{
    session::{Session, SessionFilter},
    thumbnailer::ThumbRequest,
};
use crate::db::Db;
use crate::settings::AppSettings;
use crossbeam::channel::Sender;
use std::{path::PathBuf, sync::Mutex};

pub struct AppState {
    pub session: Mutex<Option<Session>>,
    pub db: Mutex<Option<Db>>,
    /// Channel to request thumbnail / viewer decodes from the rayon pool.
    pub thumb_request_tx: Sender<ThumbRequest>,
    pub settings: Mutex<AppSettings>,
    pub filter: Mutex<SessionFilter>,
    /// Directory for cached filmstrip-quality JPEG thumbnails.
    pub thumb_cache_dir: PathBuf,
    /// Directory for cached high-res viewer JPEGs (for RAW files).
    pub viewer_cache_dir: PathBuf,
}
