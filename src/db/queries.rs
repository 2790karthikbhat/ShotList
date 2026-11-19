// src/db/queries.rs
// Typed query helpers.
//
// Persistence is keyed on (session_id, path): a file's path is its stable
// identity across re-imports, whereas the in-memory `ImageId` is just an
// ephemeral runtime handle regenerated on every scan.

use crate::core::image::{ColorLabel, CullState, ImageEntry};
use crate::core::session::Session;
use anyhow::Result;
use rusqlite::{params, Connection};
use std::collections::HashMap;

/// A previously-saved decision for one image.
#[derive(Debug, Clone)]
pub struct StoredDecision {
    pub cull_state: CullState,
    pub rating: u8,
    pub color_label: Option<ColorLabel>,
}

// ── ColorLabel DB encoding ────────────────────────────────────────────────────

fn encode_color_label(label: Option<&ColorLabel>) -> Option<&'static str> {
    label.map(|l| match l {
        ColorLabel::Red => "Red",
        ColorLabel::Orange => "Orange",
        ColorLabel::Yellow => "Yellow",
        ColorLabel::Green => "Green",
        ColorLabel::Blue => "Blue",
        ColorLabel::Purple => "Purple",
    })
}

fn decode_color_label(s: &str) -> Option<ColorLabel> {
    match s {
        "Red" => Some(ColorLabel::Red),
        "Orange" => Some(ColorLabel::Orange),
        "Yellow" => Some(ColorLabel::Yellow),
        "Green" => Some(ColorLabel::Green),
        "Blue" => Some(ColorLabel::Blue),
        "Purple" => Some(ColorLabel::Purple),
        _ => None,
    }
}

// ── Session queries ───────────────────────────────────────────────────────────

pub fn upsert_session(conn: &Connection, session: &Session) -> Result<()> {
    conn.execute(
        "INSERT INTO sessions (id, name, source_dir, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(id) DO UPDATE SET
           name       = excluded.name,
           updated_at = excluded.updated_at",
        params![
            session.id.to_string(),
            session.name,
            session.source_dir.to_string_lossy(),
            session.created_at.to_rfc3339(),
            session.updated_at.to_rfc3339(),
        ],
    )?;
    Ok(())
}

/// Find the id of a saved session whose source directory matches `dir`.
pub fn find_session_id_by_dir(conn: &Connection, dir: &str) -> Result<Option<String>> {
    let id = conn
        .query_row(
            "SELECT id FROM sessions WHERE source_dir = ?1
             ORDER BY updated_at DESC LIMIT 1",
            params![dir],
            |row| row.get::<_, String>(0),
        )
        .ok();
    Ok(id)
}

// ── Image queries ─────────────────────────────────────────────────────────────

/// Insert an image row, or — if one already exists at the same path in this
/// session — refresh its filesystem metadata. The saved decision columns
/// (`cull_state`, `rating`) and the original `id` are deliberately preserved.
pub fn upsert_image(conn: &Connection, session_id: &str, entry: &ImageEntry) -> Result<()> {
    conn.execute(
        "INSERT INTO images
           (id, session_id, path, format, file_size, modified_at,
            cull_state, rating, color_label, indexed_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
         ON CONFLICT(session_id, path) DO UPDATE SET
           format      = excluded.format,
           file_size   = excluded.file_size,
           modified_at = excluded.modified_at",
        params![
            entry.id.to_string(),
            session_id,
            entry.path.to_string_lossy(),
            format!("{:?}", entry.format),
            entry.file_size as i64,
            entry.modified_at.to_rfc3339(),
            entry.cull_state.label(),
            entry.rating as i64,
            encode_color_label(entry.color_label.as_ref()),
            entry.indexed_at.to_rfc3339(),
        ],
    )?;
    Ok(())
}

pub fn update_color_label(
    conn: &Connection,
    session_id: &str,
    path: &str,
    label: Option<&ColorLabel>,
) -> Result<()> {
    conn.execute(
        "UPDATE images SET color_label = ?1 WHERE session_id = ?2 AND path = ?3",
        params![encode_color_label(label), session_id, path],
    )?;
    Ok(())
}

pub fn update_cull_state(
    conn: &Connection,
    session_id: &str,
    path: &str,
    state: CullState,
) -> Result<()> {
    conn.execute(
        "UPDATE images SET cull_state = ?1 WHERE session_id = ?2 AND path = ?3",
        params![state.label(), session_id, path],
    )?;
    Ok(())
}

pub fn update_rating(conn: &Connection, session_id: &str, path: &str, rating: u8) -> Result<()> {
    conn.execute(
        "UPDATE images SET rating = ?1 WHERE session_id = ?2 AND path = ?3",
        params![rating as i64, session_id, path],
    )?;
    Ok(())
}

/// Load the stable DB-stored image id for every image in a session, keyed by file path.
/// Used by sync_session to restore consistent ImageIds across app restarts.
pub fn load_stable_image_ids(
    conn: &Connection,
    session_id: &str,
) -> Result<HashMap<String, String>> {
    let mut stmt = conn.prepare("SELECT path, id FROM images WHERE session_id = ?1")?;
    let rows = stmt.query_map(params![session_id], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    let mut map = HashMap::new();
    for row in rows {
        let (path, id) = row?;
        map.insert(path, id);
    }
    Ok(map)
}

/// Load every saved decision for a session, keyed by the image's file path.
pub fn load_decisions(
    conn: &Connection,
    session_id: &str,
) -> Result<HashMap<String, StoredDecision>> {
    let mut stmt = conn.prepare(
        "SELECT path, cull_state, rating, color_label
         FROM images WHERE session_id = ?1",
    )?;
    let rows = stmt.query_map(params![session_id], |row| {
        let path: String = row.get(0)?;
        let state: String = row.get(1)?;
        let rating: i64 = row.get(2)?;
        let label: Option<String> = row.get(3)?;
        Ok((
            path,
            StoredDecision {
                cull_state: CullState::from_label(&state),
                rating: rating.clamp(0, 5) as u8,
                color_label: label.as_deref().and_then(decode_color_label),
            },
        ))
    })?;

    let mut map = HashMap::new();
    for row in rows {
        let (path, decision) = row?;
        map.insert(path, decision);
    }
    Ok(map)
}

// ── Develop params queries ────────────────────────────────────────────────────

/// Persist develop params for one image (upsert by image_id).
pub fn save_develop_params(
    conn: &Connection,
    image_id: &str,
    params: &crate::core::develop::DevelopParams,
) -> Result<()> {
    let json = serde_json::to_string(params)?;
    conn.execute(
        "INSERT INTO develop_params (image_id, params_json)
         VALUES (?1, ?2)
         ON CONFLICT(image_id) DO UPDATE SET
           params_json = excluded.params_json,
           updated_at  = datetime('now')",
        params![image_id, json],
    )?;
    Ok(())
}

/// Load develop params for one image, returning `None` if never saved.
pub fn load_develop_params(
    conn: &Connection,
    image_id: &str,
) -> Result<Option<crate::core::develop::DevelopParams>> {
    let result = conn.query_row(
        "SELECT params_json FROM develop_params WHERE image_id = ?1",
        params![image_id],
        |row| row.get::<_, String>(0),
    );
    match result {
        Ok(json) => Ok(serde_json::from_str(&json).ok()),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

// ── Caption queries ───────────────────────────────────────────────────────────

/// Save (upsert) editorial caption fields for one image.
pub fn save_caption(
    conn: &Connection,
    image_id: &str,
    caption: &crate::core::caption::Caption,
) -> Result<()> {
    let keywords_json = serde_json::to_string(&caption.keywords)?;
    conn.execute(
        "INSERT INTO captions (image_id, title, description, keywords_json, location, credit, generated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(image_id) DO UPDATE SET
           title         = excluded.title,
           description   = excluded.description,
           keywords_json = excluded.keywords_json,
           location      = excluded.location,
           credit        = excluded.credit,
           generated_at  = excluded.generated_at,
           updated_at    = datetime('now')",
        params![
            image_id,
            caption.title,
            caption.description,
            keywords_json,
            caption.location,
            caption.credit,
            // generated_at is NULL unless this was AI-generated (caller sets it)
            rusqlite::types::Null,
        ],
    )?;
    Ok(())
}

/// Save caption and mark it as AI-generated at the current timestamp.
pub fn save_caption_generated(
    conn: &Connection,
    image_id: &str,
    caption: &crate::core::caption::Caption,
) -> Result<()> {
    let keywords_json = serde_json::to_string(&caption.keywords)?;
    conn.execute(
        "INSERT INTO captions (image_id, title, description, keywords_json, location, credit, generated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, datetime('now'))
         ON CONFLICT(image_id) DO UPDATE SET
           title         = excluded.title,
           description   = excluded.description,
           keywords_json = excluded.keywords_json,
           location      = excluded.location,
           credit        = excluded.credit,
           generated_at  = datetime('now'),
           updated_at    = datetime('now')",
        params![
            image_id,
            caption.title,
            caption.description,
            keywords_json,
            caption.location,
            caption.credit,
        ],
    )?;
    Ok(())
}

/// Load caption for one image. Returns `None` if no caption saved yet.
pub fn load_caption(
    conn: &Connection,
    image_id: &str,
) -> Result<Option<crate::core::caption::Caption>> {
    let result = conn.query_row(
        "SELECT title, description, keywords_json, location, credit
         FROM captions WHERE image_id = ?1",
        params![image_id],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
            ))
        },
    );
    match result {
        Ok((title, description, keywords_json, location, credit)) => {
            let keywords = serde_json::from_str::<Vec<String>>(&keywords_json).unwrap_or_default();
            Ok(Some(crate::core::caption::Caption {
                title,
                description,
                keywords,
                location,
                credit,
            }))
        }
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Load captions for all images in a session. Returns a map of image_id → Caption.
pub fn load_all_captions(
    conn: &Connection,
    image_ids: &[String],
) -> Result<HashMap<String, crate::core::caption::Caption>> {
    let mut map = HashMap::new();
    for id in image_ids {
        if let Ok(Some(caption)) = load_caption(conn, id) {
            map.insert(id.clone(), caption);
        }
    }
    Ok(map)
}

// ── Zine queries ──────────────────────────────────────────────────────────────

use crate::core::zine::{PaperSize, TemplateId, Zine, ZinePage};

/// Create a new zine for a session (or return existing one if it already exists).
pub fn create_or_get_zine(conn: &Connection, session_id: &str) -> Result<Zine> {
    // Check if one already exists.
    let existing: Option<(String, String)> = conn
        .query_row(
            "SELECT id, paper_size FROM zines WHERE session_id = ?1 ORDER BY created_at LIMIT 1",
            params![session_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .ok();

    let (zine_id, paper_size_str) = if let Some(row) = existing {
        row
    } else {
        let id = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO zines (id, session_id, paper_size) VALUES (?1, ?2, 'a5')",
            params![id, session_id],
        )?;
        (id, "a5".to_string())
    };

    let paper_size = PaperSize::from_label(&paper_size_str);
    let pages = load_zine_pages(conn, &zine_id)?;

    Ok(Zine {
        id: zine_id,
        session_id: session_id.to_string(),
        paper_size,
        pages,
    })
}

/// Load all pages for a zine, ordered by position.
pub fn load_zine_pages(conn: &Connection, zine_id: &str) -> Result<Vec<ZinePage>> {
    let mut stmt = conn.prepare(
        "SELECT id, zine_id, position, image_id, template, caption_override,
                image_id_2, image_id_3, image_id_4,
                image_id_5, image_id_6, image_id_7, image_id_8, image_id_9
         FROM zine_pages WHERE zine_id = ?1 ORDER BY position",
    )?;
    let rows = stmt.query_map(params![zine_id], |row| {
        let template_str: String = row.get(4)?;
        let template = TemplateId::from_label(&template_str);
        Ok(ZinePage {
            id: row.get(0)?,
            zine_id: row.get(1)?,
            position: row.get(2)?,
            image_id: row.get(3)?,
            template,
            caption_override: row.get(5)?,
            image_id_2: row.get(6)?,
            image_id_3: row.get(7)?,
            image_id_4: row.get(8)?,
            image_id_5: row.get(9)?,
            image_id_6: row.get(10)?,
            image_id_7: row.get(11)?,
            image_id_8: row.get(12)?,
            image_id_9: row.get(13)?,
        })
    })?;

    let mut pages = Vec::new();
    for row in rows {
        pages.push(row?);
    }
    Ok(pages)
}

/// Add a page to a zine.
pub fn add_zine_page(
    conn: &Connection,
    zine_id: &str,
    position: i64,
    image_id: Option<&str>,
    template: &TemplateId,
) -> Result<ZinePage> {
    let page_id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO zine_pages (id, zine_id, position, image_id, template)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![page_id, zine_id, position, image_id, template.label()],
    )?;
    Ok(ZinePage {
        id: page_id,
        zine_id: zine_id.to_string(),
        position,
        image_id: image_id.map(|s| s.to_string()),
        image_id_2: None,
        image_id_3: None,
        image_id_4: None,
        image_id_5: None,
        image_id_6: None,
        image_id_7: None,
        image_id_8: None,
        image_id_9: None,
        template: template.clone(),
        caption_override: None,
    })
}

/// Reorder pages by updating their position values from a new ordered list of page IDs.
pub fn update_page_order(conn: &Connection, ordered_page_ids: &[String]) -> Result<()> {
    for (i, page_id) in ordered_page_ids.iter().enumerate() {
        conn.execute(
            "UPDATE zine_pages SET position = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![i as i64, page_id],
        )?;
    }
    Ok(())
}

/// Set the template for a single page.
pub fn set_page_template(conn: &Connection, page_id: &str, template: &TemplateId) -> Result<()> {
    conn.execute(
        "UPDATE zine_pages SET template = ?1, updated_at = datetime('now') WHERE id = ?2",
        params![template.label(), page_id],
    )?;
    Ok(())
}

/// Set (or clear) the caption override text for a page.
pub fn set_page_caption_override(
    conn: &Connection,
    page_id: &str,
    override_: Option<&str>,
) -> Result<()> {
    conn.execute(
        "UPDATE zine_pages SET caption_override = ?1, updated_at = datetime('now') WHERE id = ?2",
        params![override_, page_id],
    )?;
    Ok(())
}

/// Remove a single page from a zine.
pub fn remove_zine_page(conn: &Connection, page_id: &str) -> Result<()> {
    conn.execute("DELETE FROM zine_pages WHERE id = ?1", params![page_id])?;
    Ok(())
}

/// Set (or clear) the image on a zine page.
pub fn set_page_image(conn: &Connection, page_id: &str, image_id: Option<&str>) -> Result<()> {
    conn.execute(
        "UPDATE zine_pages SET image_id = ?1, updated_at = datetime('now') WHERE id = ?2",
        params![image_id, page_id],
    )?;
    Ok(())
}

/// Set (or clear) image slot 2, 3, or 4 on a page.
pub fn set_page_image_slot(
    conn: &Connection,
    page_id: &str,
    slot: u8,
    image_id: Option<&str>,
) -> Result<()> {
    let col = match slot {
        2 => "image_id_2",
        3 => "image_id_3",
        4 => "image_id_4",
        5 => "image_id_5",
        6 => "image_id_6",
        7 => "image_id_7",
        8 => "image_id_8",
        9 => "image_id_9",
        _ => return set_page_image(conn, page_id, image_id),
    };
    conn.execute(
        &format!("UPDATE zine_pages SET {col} = ?1, updated_at = datetime('now') WHERE id = ?2"),
        params![image_id, page_id],
    )?;
    Ok(())
}

/// Update the paper size of a zine.
pub fn set_zine_paper_size(conn: &Connection, zine_id: &str, size: &PaperSize) -> Result<()> {
    conn.execute(
        "UPDATE zines SET paper_size = ?1, updated_at = datetime('now') WHERE id = ?2",
        params![size.label(), zine_id],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migrations;

    fn setup_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        migrations::run(&conn).unwrap();
        conn
    }

    fn insert_session(conn: &Connection, id: &str) {
        conn.execute(
            "INSERT INTO sessions (id, name, source_dir, created_at, updated_at)
             VALUES (?1, 'test', '/tmp/test', datetime('now'), datetime('now'))",
            params![id],
        )
        .unwrap();
    }

    #[test]
    fn migrations_create_expected_tables() {
        let conn = setup_db();
        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap();
        let tables: Vec<String> = stmt
            .query_map([], |r| r.get(0))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();

        for expected in [
            "sessions",
            "images",
            "ai_assessments",
            "develop_params",
            "captions",
            "zines",
            "zine_pages",
        ] {
            assert!(
                tables.iter().any(|t| t == expected),
                "missing table {expected}"
            );
        }
    }

    #[test]
    fn migrations_add_all_multi_image_slot_columns() {
        let conn = setup_db();
        let mut stmt = conn.prepare("PRAGMA table_info(zine_pages)").unwrap();
        let cols: Vec<String> = stmt
            .query_map([], |r| r.get::<_, String>(1))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();

        for slot in 2..=9 {
            let col = format!("image_id_{slot}");
            assert!(cols.contains(&col), "missing column {col}");
        }
    }

    #[test]
    fn migrations_are_idempotent() {
        let conn = setup_db();
        // Re-running must not error — ALTER TABLE isn't naturally idempotent,
        // so this guards the schema_version skip logic in migrations::run.
        migrations::run(&conn).unwrap();
    }

    #[test]
    fn set_page_image_slot_round_trips_all_nine_slots() {
        let conn = setup_db();
        insert_session(&conn, "sess-1");
        let zine = create_or_get_zine(&conn, "sess-1").unwrap();
        let page = add_zine_page(&conn, &zine.id, 0, None, &TemplateId::GridTwo).unwrap();

        for slot in 1..=9u8 {
            let image_id = format!("img-{slot}");
            set_page_image_slot(&conn, &page.id, slot, Some(image_id.as_str())).unwrap();
        }

        let pages = load_zine_pages(&conn, &zine.id).unwrap();
        let reloaded = pages.into_iter().find(|p| p.id == page.id).unwrap();

        assert_eq!(reloaded.image_id.as_deref(), Some("img-1"));
        assert_eq!(reloaded.image_id_2.as_deref(), Some("img-2"));
        assert_eq!(reloaded.image_id_3.as_deref(), Some("img-3"));
        assert_eq!(reloaded.image_id_4.as_deref(), Some("img-4"));
        assert_eq!(reloaded.image_id_5.as_deref(), Some("img-5"));
        assert_eq!(reloaded.image_id_6.as_deref(), Some("img-6"));
        assert_eq!(reloaded.image_id_7.as_deref(), Some("img-7"));
        assert_eq!(reloaded.image_id_8.as_deref(), Some("img-8"));
        assert_eq!(reloaded.image_id_9.as_deref(), Some("img-9"));
    }

    #[test]
    fn set_page_image_slot_clears_a_slot() {
        let conn = setup_db();
        insert_session(&conn, "sess-2");
        let zine = create_or_get_zine(&conn, "sess-2").unwrap();
        let page = add_zine_page(&conn, &zine.id, 0, None, &TemplateId::Triptych).unwrap();

        set_page_image_slot(&conn, &page.id, 2, Some("img-x")).unwrap();
        set_page_image_slot(&conn, &page.id, 2, None).unwrap();

        let pages = load_zine_pages(&conn, &zine.id).unwrap();
        let reloaded = pages.into_iter().find(|p| p.id == page.id).unwrap();
        assert_eq!(reloaded.image_id_2, None);
    }
}
