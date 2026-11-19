# Navigation & Folder Browser

| File | Task | One-liner | Status | Pending items |
|---|---|---|---|---|
| `frontend/src/components/icons.tsx` | Icon system | Started as a hand-rolled inline-SVG set, later migrated to thin `lucide-react` re-exports (already a dependency via ZineStage) under the same names, for consistent, properly-drawn icons and no more sizing bugs | ✅ Done | — |
| `frontend/src/components/StageNav.tsx` | Nav rail redesign | Icon-based stage list, DOM-measured sliding active-pill indicator (not a hardcoded pixel offset — this app's 13px root font-size broke that assumption once), collapsible icon+label expand/collapse that correctly fits the ~39px collapsed rail | ✅ Done | — |
| `frontend/src/App.tsx`, `index.css` | Stage transitions | CSS fade/slide-in on stage switch via `key={stage}` remount | ✅ Done | — |
| `src/commands.rs`, `src/settings.rs` | Folder sidebar backend | `open_folder_at`, `list_dir_entries` (folders + image files), `get_root_locations`, `preview_file`, `recent_folders` (record + user-removable) | ✅ Done | — |
| `frontend/src/components/FolderSidebar.tsx` | Folder sidebar UI | Collapsible tree — Recents (removable via hover ×), Home/Desktop/Pictures lazy-expanding tree; single click opens a folder immediately (chevron is the separate expand/collapse control); Quick Look overlay previews an individual photo without importing its folder; "Open in Develop" imports the containing folder and jumps straight to that photo | ✅ Done | — |
| `frontend/src/store.ts` | Session-loading refactor | Shared `applySession` helper (with `stage`/`focusPath` overrides) used by `importFolder`, `openFolderAt`, and `openFolderAtImage` | ✅ Done | — |
| `frontend/src/components/Filmstrip.tsx` | Thumbnail-request bug fix | Effect no longer depends on the whole `thumbnails` object (was re-requesting the entire still-missing set on every single thumbnail arrival, flooding the backend's decode channel until it silently dropped unrelated requests) | ✅ Done | — |
| `frontend/src/store.ts` | Filter-switch bug fix | `applyFilter` now re-derives `currentId` when the previously-selected image is filtered out, instead of leaving it pointing at an id no longer in the new visible set (viewer going blank) | ✅ Done | — |

## Notes
- Quick Look decodes RAW via the same fast embedded-preview path the thumbnailer already uses, independent of any open session.
- Toolbar header icons across all 5 stages were unified to match the nav rail (previously mismatched unicode glyphs).
