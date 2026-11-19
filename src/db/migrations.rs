// src/db/migrations.rs
// Embedded SQL migrations. Each migration runs exactly once, tracked by version.

use anyhow::Result;
use rusqlite::Connection;
use tracing::info;

/// All migrations in order. Version = 1-indexed position in this array.
const MIGRATIONS: &[&str] = &[
    // V1 — initial schema
    r#"
    CREATE TABLE IF NOT EXISTS schema_version (
        version  INTEGER PRIMARY KEY,
        applied_at TEXT NOT NULL DEFAULT (datetime('now'))
    );

    CREATE TABLE IF NOT EXISTS sessions (
        id          TEXT PRIMARY KEY,
        name        TEXT NOT NULL,
        source_dir  TEXT NOT NULL,
        created_at  TEXT NOT NULL,
        updated_at  TEXT NOT NULL
    );

    CREATE TABLE IF NOT EXISTS images (
        id              TEXT PRIMARY KEY,
        session_id      TEXT NOT NULL REFERENCES sessions(id),
        path            TEXT NOT NULL,
        format          TEXT NOT NULL,
        file_size       INTEGER NOT NULL,
        modified_at     TEXT NOT NULL,
        captured_at     TEXT,
        iso             INTEGER,
        shutter_speed   TEXT,
        aperture        REAL,
        cull_state      TEXT NOT NULL DEFAULT 'unreviewed',
        rating          INTEGER NOT NULL DEFAULT 0,
        color_label     TEXT,
        indexed_at      TEXT NOT NULL,
        UNIQUE(session_id, path)
    );

    CREATE TABLE IF NOT EXISTS ai_assessments (
        image_id        TEXT PRIMARY KEY REFERENCES images(id),
        focus_score     REAL,
        scene_rank      REAL,
        ai_label        TEXT,
        provider        TEXT,
        faces_json      TEXT,  -- JSON array of FaceRegion
        assessed_at     TEXT NOT NULL DEFAULT (datetime('now'))
    );

    CREATE INDEX IF NOT EXISTS idx_images_session ON images(session_id);
    CREATE INDEX IF NOT EXISTS idx_images_cull_state ON images(session_id, cull_state);
    "#,
    // V2 — develop params (non-destructive editing pipeline per image)
    r#"
    CREATE TABLE IF NOT EXISTS develop_params (
        image_id    TEXT PRIMARY KEY,
        params_json TEXT NOT NULL,
        updated_at  TEXT NOT NULL DEFAULT (datetime('now'))
    );
    "#,
    // V3 — editorial captions (title, description, keywords, location, credit)
    r#"
    CREATE TABLE IF NOT EXISTS captions (
        image_id      TEXT PRIMARY KEY,
        title         TEXT NOT NULL DEFAULT '',
        description   TEXT NOT NULL DEFAULT '',
        keywords_json TEXT NOT NULL DEFAULT '[]',
        location      TEXT NOT NULL DEFAULT '',
        credit        TEXT NOT NULL DEFAULT '',
        generated_at  TEXT,
        updated_at    TEXT NOT NULL DEFAULT (datetime('now'))
    );
    "#,
    // V4 — zine builder (one zine per session, N ordered pages)
    r#"
    CREATE TABLE IF NOT EXISTS zines (
        id          TEXT PRIMARY KEY,
        session_id  TEXT NOT NULL REFERENCES sessions(id),
        paper_size  TEXT NOT NULL DEFAULT 'a5',
        created_at  TEXT NOT NULL DEFAULT (datetime('now')),
        updated_at  TEXT NOT NULL DEFAULT (datetime('now'))
    );

    CREATE TABLE IF NOT EXISTS zine_pages (
        id               TEXT PRIMARY KEY,
        zine_id          TEXT NOT NULL REFERENCES zines(id) ON DELETE CASCADE,
        position         INTEGER NOT NULL,
        image_id         TEXT,
        template         TEXT NOT NULL DEFAULT 'top75',
        caption_override TEXT,
        updated_at       TEXT NOT NULL DEFAULT (datetime('now'))
    );

    CREATE INDEX IF NOT EXISTS idx_zine_pages_zine ON zine_pages(zine_id, position);
    "#,
    // V2 — multi-image slots for Triptych, Grid4x, FeatureSpread, Editorial
    r#"
    ALTER TABLE zine_pages ADD COLUMN image_id_2 TEXT;
    ALTER TABLE zine_pages ADD COLUMN image_id_3 TEXT;
    ALTER TABLE zine_pages ADD COLUMN image_id_4 TEXT;
    "#,
    // V3 — extra slots for PhotoEssay (5) and ContactSheet (9)
    r#"
    ALTER TABLE zine_pages ADD COLUMN image_id_5 TEXT;
    ALTER TABLE zine_pages ADD COLUMN image_id_6 TEXT;
    ALTER TABLE zine_pages ADD COLUMN image_id_7 TEXT;
    ALTER TABLE zine_pages ADD COLUMN image_id_8 TEXT;
    ALTER TABLE zine_pages ADD COLUMN image_id_9 TEXT;
    "#,
];

pub fn run(conn: &Connection) -> Result<()> {
    // Ensure the version table exists (bootstrapping problem).
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER PRIMARY KEY,
            applied_at TEXT NOT NULL DEFAULT (datetime('now'))
        );",
    )?;

    let current_version: i64 = conn.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_version",
        [],
        |r| r.get(0),
    )?;

    for (i, sql) in MIGRATIONS.iter().enumerate() {
        let version = (i + 1) as i64;
        if version <= current_version {
            continue;
        }

        info!("Applying DB migration V{version}");
        conn.execute_batch(sql)?;
        conn.execute(
            "INSERT INTO schema_version (version) VALUES (?1)",
            rusqlite::params![version],
        )?;
    }

    Ok(())
}
