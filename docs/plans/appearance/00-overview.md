# Appearance (Light / Dark / System Theme)

| File | Task | One-liner | Status | Pending items |
|---|---|---|---|---|
| `frontend/src/index.css` | CSS variable system | `:root` (dark) + `[data-theme="light"]` override block; RGB-triplet vars (not hex) so Tailwind's `bg-x/50`-style opacity modifiers keep working | ✅ Done | — |
| `frontend/tailwind.config.js` | Token wiring | `bg`/`panel`/`surface`/`border`/`hi`/`mid`/`lo`/`accent` point at the CSS vars; `pick`/`reject`/`flag` stay literal (confirmed theme-independent) | ✅ Done | — |
| `frontend/src/lib/theme.ts` | Theme application | `applyTheme()` resolves `system` via `matchMedia`, stays live-reactive to OS appearance changes while active | ✅ Done | — |
| `src/settings.rs`, `frontend/src/types.ts` | Persisted setting | `AppSettings.theme: dark \| light \| system`, defaults to `dark` (zero visual change for existing users) | ✅ Done | — |
| `frontend/src/App.tsx` | Settings UI | New "Appearance" section (Dark/Light/System segmented control) at the top of the Settings modal | ✅ Done | — |
| `Toast.tsx`, `Filmstrip.tsx`, `CullStage.tsx`, `AnalyseStage.tsx` | Targeted color fixes | Dark-assuming literals given light counterparts: toast tint backgrounds, filmstrip placeholder cell, app-mark glyph, Kbd keys, empty-star color, canvas-drawn histogram (read via `getComputedStyle` at draw-time since `<canvas>` can't use CSS variables) | ✅ Done | — |
| `frontend/src/stages/ZineStage.tsx` | Zine conversion | ~50 hardcoded `neutral-*` chrome classes converted to the shared tokens (toolbar, page rail, canvas frame, inspector panel, picker modal) | ✅ Done | — |

## Design notes
- Covers Cull, Analyse, Develop, Caption, Zine, and all shared chrome (nav, folder sidebar, settings, toast, filmstrip) — full app coverage.
- The "printed zine page" mock and template-picker diagram icons in Zine are deliberately left theme-independent — they simulate physical paper/photos, not app chrome, same as a real page-layout tool's canvas staying paper-colored regardless of the app's own UI theme.
- Incidental fix found along the way: `DevelopStage`'s slider-track gradient referenced `var(--color-accent)`/`var(--color-surface)`, which never actually existed anywhere before this work (silent no-op) — now correctly defined.
- Incidental fix: a `hover:bg-neutral-750` in Zine wasn't a real Tailwind shade (default palette has no 750) — was a dead hover state, now works.
