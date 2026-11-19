// tests/mvp_workflow.rs
// Integration tests for the JPEG MVP workflow: scan → persist → resume → export.
//
// These exercise the non-UI logic. File contents are irrelevant here because
// the import / persist / export paths never decode the images — only the
// thumbnailer does, and that is not under test.

use shotlist::core::image::CullState;
use shotlist::core::{export, import};
use shotlist::db::{queries, Db};
use std::fs;
use std::path::Path;

fn touch(path: &Path) {
    fs::write(path, b"not-a-real-image").expect("create test file");
}

#[test]
fn import_finds_images_and_skips_other_files() {
    let dir = tempfile::tempdir().unwrap();
    touch(&dir.path().join("a.jpg"));
    touch(&dir.path().join("b.JPEG")); // case-insensitive extension
    touch(&dir.path().join("c.tif"));
    touch(&dir.path().join("notes.txt"));
    touch(&dir.path().join("archive.zip"));

    let session = import::session_from_folder(dir.path().to_path_buf());

    assert_eq!(
        session.image_ids.len(),
        3,
        "should find a.jpg, b.JPEG and c.tif, skipping the txt and zip"
    );
}

#[test]
fn cull_decisions_resume_after_reimport() {
    let dir = tempfile::tempdir().unwrap();
    touch(&dir.path().join("keep.jpg"));
    touch(&dir.path().join("toss.jpg"));

    // First pass: import, persist, then mark one Pick and one Reject.
    {
        let mut session = import::session_from_folder(dir.path().to_path_buf());
        let db = Db::open(dir.path()).unwrap();
        db.sync_session(&mut session).unwrap();

        let sid = session.id.to_string();
        for id in session.image_ids.clone() {
            let entry = &session.images[&id];
            let state = if entry.filename() == "keep.jpg" {
                CullState::Pick
            } else {
                CullState::Reject
            };
            queries::update_cull_state(&db.conn, &sid, &entry.path.to_string_lossy(), state)
                .unwrap();
        }
    }

    // Second pass: a fresh import of the same folder must resume the decisions.
    {
        let mut session = import::session_from_folder(dir.path().to_path_buf());
        let db = Db::open(dir.path()).unwrap();
        db.sync_session(&mut session).unwrap();

        let stats = session.stats();
        assert_eq!(stats.picks, 1, "the pick should be restored");
        assert_eq!(stats.rejects, 1, "the reject should be restored");
        assert_eq!(stats.unreviewed, 0, "nothing should remain unreviewed");
    }
}

#[test]
fn export_copies_only_picked_images() {
    let dir = tempfile::tempdir().unwrap();
    touch(&dir.path().join("one.jpg"));
    touch(&dir.path().join("two.jpg"));
    touch(&dir.path().join("three.jpg"));

    let mut session = import::session_from_folder(dir.path().to_path_buf());
    let ids = session.image_ids.clone();
    session.set_cull_state(ids[0], CullState::Pick);
    session.set_cull_state(ids[1], CullState::Pick);
    // ids[2] is left Unreviewed.

    let summary = export::export_picks(&session).unwrap();
    assert_eq!(summary.copied, 2, "only the two picks should be copied");

    let picks: Vec<_> = fs::read_dir(dir.path().join("Picks"))
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert_eq!(picks.len(), 2, "Picks/ should hold exactly two files");
}
