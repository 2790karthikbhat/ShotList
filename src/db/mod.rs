// src/db/mod.rs
// SQLite persistence layer.
// One `.culld.db` file lives alongside the source image folder.

pub mod migrations;
pub mod queries;

use crate::core::image::ImageId;
use crate::core::session::{Session, SessionId};
use anyhow::{Context, Result};
use rusqlite::Connection;
use std::collections::HashMap;
use std::path::Path;
use tracing::{info, warn};

/// Open (or create) the database for a project directory.
pub fn open(project_dir: &Path) -> Result<Connection> {
    let db_path = project_dir.join(".shotlist.db");
    info!("Opening database at {:?}", db_path);

    let conn = Connection::open(&db_path)
        .with_context(|| format!("Failed to open database at {db_path:?}"))?;

    // WAL mode: better concurrent read performance.
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;

    // Run pending schema migrations.
    migrations::run(&conn)?;

    Ok(conn)
}

/// A project database handle.
pub struct Db {
    pub conn: Connection,
}

impl Db {
    pub fn open(project_dir: &Path) -> Result<Self> {
        Ok(Self {
            conn: open(project_dir)?,
        })
    }

    /// Merge any previously-saved decisions for this folder onto a freshly
    /// scanned `session`, then persist the session and all its images.
    ///
    /// Images are matched by file path, so cull states and star ratings
    /// survive re-imports even though every scan generates new in-memory ids.
    pub fn sync_session(&self, session: &mut Session) -> Result<()> {
        let dir = session.source_dir.to_string_lossy().to_string();

        // Reuse the saved session id for this directory, if one exists, so
        // previously-stored image rows still belong to the same session.
        if let Some(existing) = queries::find_session_id_by_dir(&self.conn, &dir)? {
            match SessionId::parse(&existing) {
                Some(parsed) => session.id = parsed,
                None => warn!("Stored session id {existing:?} is invalid; starting fresh"),
            }
        }

        // Apply stored cull states / ratings onto the scanned images.
        let session_id = session.id.to_string();
        let decisions = queries::load_decisions(&self.conn, &session_id)?;
        let stable_ids = queries::load_stable_image_ids(&self.conn, &session_id)?;

        let mut resumed = 0;
        for entry in session.images.values_mut() {
            let key = entry.path.to_string_lossy().to_string();
            if let Some(d) = decisions.get(&key) {
                entry.cull_state = d.cull_state;
                entry.rating = d.rating;
                entry.color_label = d.color_label;
                resumed += 1;
            }
        }
        if resumed > 0 {
            info!("Resumed {resumed} saved decision(s) for {dir:?}");
        }

        // Remap in-memory ImageIds to match the DB-stored stable IDs so that any
        // persisted references (e.g. zine_pages.image_id) survive across restarts.
        if !stable_ids.is_empty() {
            let mut remap: HashMap<ImageId, ImageId> = HashMap::new();
            let mut new_images = HashMap::with_capacity(session.images.len());

            for (old_id, mut entry) in session.images.drain() {
                let path_key = entry.path.to_string_lossy().to_string();
                if let Some(stored_str) = stable_ids.get(&path_key) {
                    if let Ok(stable_id) = stored_str.parse::<ImageId>() {
                        remap.insert(old_id, stable_id);
                        entry.id = stable_id;
                        new_images.insert(stable_id, entry);
                        continue;
                    }
                }
                new_images.insert(old_id, entry);
            }

            session.images = new_images;

            for id in &mut session.image_ids {
                if let Some(&stable) = remap.get(id) {
                    *id = stable;
                }
            }
        }

        // Persist the merged session + all images.
        queries::upsert_session(&self.conn, session)?;
        for entry in session.images.values() {
            queries::upsert_image(&self.conn, &session_id, entry)?;
        }
        Ok(())
    }
}
