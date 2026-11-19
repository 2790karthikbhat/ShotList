// src/core/watcher.rs
// Watches the session source directory and emits events when new images appear.
// Uses the `notify` crate which wraps FSEvents (macOS), inotify (Linux), and
// ReadDirectoryChangesW (Windows).

use crate::core::image::ImageFormat;
use anyhow::Result;
use crossbeam::channel::{self, Receiver};
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use tracing::{debug, warn};

#[derive(Debug)]
pub enum FolderEvent {
    /// A new image file was created in the watched directory.
    ImageAdded(PathBuf),
    /// An existing image file was removed.
    ImageRemoved(PathBuf),
}

pub struct FolderWatcher {
    pub events: Receiver<FolderEvent>,
    // Keep the watcher alive — dropping it stops watching.
    _watcher: RecommendedWatcher,
}

impl FolderWatcher {
    pub fn watch(dir: PathBuf) -> Result<Self> {
        let (event_tx, event_rx) = channel::bounded(64);

        let mut watcher = RecommendedWatcher::new(
            move |res: notify::Result<Event>| match res {
                Ok(event) => {
                    for path in event.paths {
                        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

                        if ImageFormat::from_extension(ext).is_none() {
                            continue; // skip non-image files
                        }

                        let folder_event = match event.kind {
                            EventKind::Create(_) => FolderEvent::ImageAdded(path),
                            EventKind::Remove(_) => FolderEvent::ImageRemoved(path),
                            _ => continue,
                        };

                        debug!("Folder event: {:?}", folder_event);
                        let _ = event_tx.try_send(folder_event);
                    }
                }
                Err(e) => warn!("Watcher error: {e}"),
            },
            Config::default(),
        )?;

        watcher.watch(&dir, RecursiveMode::NonRecursive)?;

        Ok(Self {
            events: event_rx,
            _watcher: watcher,
        })
    }

    /// Non-blocking drain — call this from the UI loop each frame.
    pub fn poll(&self) -> impl Iterator<Item = FolderEvent> + '_ {
        std::iter::from_fn(|| self.events.try_recv().ok())
    }
}
