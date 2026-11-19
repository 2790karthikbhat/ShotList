# Rotate & Crop

| File | Task | One-liner | Status | Pending items |
|---|---|---|---|---|
| `src/core/develop.rs` | `CropRect` + `DevelopParams` fields | `rotation:i16`, `crop:Option<CropRect>`, default identity | ✅ Done | — |
| `src/core/develop.rs` | Geometry step | `apply_geometry`: rotate then crop, before the color pipeline | ✅ Done | — |
| `src/core/develop.rs` | Unit tests | Dimension checks for rotate/crop/rotate+crop | ✅ Done | — |
| `frontend/src/types.ts` | `CropRect` + `DevelopParams` mirror | TS types + `DEFAULT_DEVELOP_PARAMS` | ✅ Done | — |
| `frontend/src/stages/DevelopStage.tsx` | Geometry section | Rotate left/right buttons, crop toggle, aspect presets, reset crop | ✅ Done | — |
| `frontend/src/stages/DevelopStage.tsx` | Crop overlay | Draggable/resizable rect over the live preview, orientation-aware presets (Free, 1:1, 4:3, 3:2, 16:9, 4:1) | ✅ Done | — |
| — | Manual verification | Exercise rotate/crop/export/persistence via `npx tauri dev` | ✅ Done | — |
| `src/commands.rs`, `src/main.rs` | Propagate to other stages | `_dev` cache tier regenerated on save, preferred by `get_image_src`/`get_zine_thumb_urls`/`emit_thumbnail`, so Cull/Analyse/Caption/Zine all reflect saved geometry immediately | ✅ Done | — |
| `src/commands.rs` | RAW race fix | Regeneration decodes straight from source instead of depending on the async pristine-viewer cache (was silently skipping regeneration right after a quick edit-then-save) | ✅ Done | — |
| `frontend/src/stages/DevelopStage.tsx` | Image-switch flash fix | Immediate non-debounced fetch on navigation + dedicated `beforeSrc` for Before/After | ✅ Done | — |

## Verification checklist
- Rotate an image both directions back to identity (0° after 4×90°).
- Apply each crop preset on both a landscape and a portrait source image; confirm the preset auto-flips orientation.
- Drag-resize a free-form crop from all 8 handles and by moving the whole rect.
- Confirm Before/After toggle and Export both reflect the crop/rotation.
- Switch images and back; confirm saved rotation/crop reload correctly.
