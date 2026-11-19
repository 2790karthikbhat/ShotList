# Release Hardening

Cross-cutting workstream covering the findings from the 2026-07-21 codebase
review: packaging config, a real Zine template bug, quality-gate drift,
secret storage, and the test-coverage gap (Rust and frontend). Unlike the
other `docs/plans/<feature>/` folders, this one isn't a single user-facing
feature — it's what's left before a distributable release. Grouped into
workstreams below; see "Execution order" for the sequence agreed with the
user.

## Release packaging

| File | Task | One-liner | Status | Pending items |
|---|---|---|---|---|
| `tauri.conf.json` | Bundle identifier | `com.shotlist.app` ends in `.app`, which Tauri itself warns conflicts with the macOS bundle extension | ✅ Done | Changed to `com.karthickbhat.shotlist` |
| `tauri.conf.json` | ATS exception domain | `macOS.exceptionDomain: ""` produces a malformed empty-string `<key></key>` in the generated `Info.plist`'s `NSExceptionDomains` | ✅ Done | Key removed — confirmed nothing in the WKWebView makes HTTP requests (LM Studio calls go through Rust's `reqwest`/`ureq`, not `fetch()`, so ATS never applies) |
| — | Code signing & notarization | `signingIdentity: null` — fine for local `tauri build`, but ad-hoc signing won't pass Gatekeeper for distribution outside this machine | ⏳ Blocked/deferred | **Decision made:** left as-is for now — this is a personal/local-use app today, and an Apple Developer Program membership ($99/yr) isn't worth it until there's an actual need to share the DMG with others. Revisit (Developer ID cert + notarization, or Mac App Store) if/when distribution beyond this machine becomes a real need |

## Zine — gridTwo template

Tracked here (not in `zine/00-overview.md`) because it's a regression fix,
not new scope. **Zine's overall status in `README.md`/`INDEX.md` should
revert to 🟡 until this lands** — the template is user-selectable today but
silently drops the second image on both PDF export and PDF preview.

| File | Task | One-liner | Status | Pending items |
|---|---|---|---|---|
| `src/core/pdf.rs:512` | GridTwo PDF render | Only places `raw` (primary image) on the left half; right half is left blank | ✅ Done | Now loops both `image_id`/`image_id_2` with `place_image_cover`, matching the Triptych/Grid4x pattern, with a grey placeholder fill + white divider when a slot is empty |
| `frontend/src/stages/ZineStage.tsx:944` | Inspector extra-slot control | `gridTwo` is missing from the multi-image template allowlist, so there's no UI to assign a second image | ✅ Done | Added to the allowlist with `['Left', 'Right']` slot labels |
| `frontend/src/stages/ZineStage.tsx:306` | Live preview | Right half renders a static `<ImageIcon>` placeholder instead of `imageId2`'s thumbnail | ✅ Done | Both panels now render via the same `t(id)` thumb lookup used by Triptych/Grid4x; placeholder icon only shows when a slot is genuinely empty |

## Quality gates

| File | Task | One-liner | Status | Pending items |
|---|---|---|---|---|
| `src/**/*.rs` | `cargo fmt` | `cargo fmt --check` currently fails with real diffs in `commands.rs` (and touches `pdf.rs`/`zine.rs` per the review) | ✅ Done | Ran `cargo fmt`; diff was whitespace-only (verified via `cargo test`, all 28 still pass) |
| `src/**/*.rs` | `cargo clippy -D warnings` | `cargo clippy --all-targets --all-features -- -D warnings` fails to compile: 18 mechanical lints across `sequence.rs`, `commands.rs`, `burst.rs`, `develop.rs`, `pdf.rs`, `thumbnailer.rs`, `xmp.rs`, `db/mod.rs`, `main.rs` | ✅ Done | All fixed individually (slice instead of `&mut Vec`, `is_some_and`, `.clamp()`, dropped a discarded `.enumerate()`, `(1_u32..).zip()` instead of a manual counter, struct-literal init in 4 develop.rs tests, `&Path` instead of `&PathBuf`, range pattern, `is_ok_and`, dropped a needless `.clone()` on a `Copy` type, plain fn ref instead of a redundant closure). `push_text_block` (`src/core/pdf.rs`, 9 args, 25 call sites) got `#[allow(clippy::too_many_arguments)]` with a comment — a params-struct rewrite across 25 sites was judged higher-risk than the lint it silences, for a private helper |
| `.pre-commit-config.yaml` → `.githooks/pre-commit` | Pre-commit hook is non-functional | Old config referenced `fmt`/`clippy` hook ids from `pre-commit/pre-commit-hooks` v2.3.0 (doesn't define Rust hooks); the `pre-commit` Python tool isn't even installed on this machine, so it never ran | ✅ Done | Replaced with a plain shell hook at `.githooks/pre-commit` (`cargo fmt --check` + `cargo clippy --all-targets --all-features -- -D warnings`) — zero new external dependencies. Activated via `git config core.hooksPath .githooks`. The existing personal `post-commit` hook (Obsidian note logging) was copied into `.githooks/post-commit` too, so switching `core.hooksPath` doesn't silently drop it. Old `.pre-commit-config.yaml` deleted |

## Security

| File | Task | One-liner | Status | Pending items |
|---|---|---|---|---|
| `src/settings.rs` | API key storage | Anthropic/OpenAI keys persist as plaintext JSON at `~/Library/Application Support/shotlist/settings.json` | ⏳ Blocked/deferred | **Parked** — folding into a later AI-provider suite that also covers OpenAI and general provider/API-key handling, rather than doing keychain migration in isolation |

## AI providers

Already tracked in [ai-providers/00-overview.md](../ai-providers/00-overview.md) — Gemini has a `Cargo.toml` feature flag but no `src/ai/gemini.rs`. Status: ⏳ **Blocked/deferred** — parked alongside Security above, to be decided (implement vs. drop) as part of the same later AI-provider suite covering Anthropic/OpenAI/Gemini/LM Studio together, not in isolation.

## Rust test coverage

| File | Task | One-liner | Status | Pending items |
|---|---|---|---|---|
| `src/db/queries.rs`, `src/db/migrations.rs` | Migration + zine-slot persistence tests | Zero `#[test]` coverage anywhere in `src/db/` today | ✅ Done | 5 tests added: migrations create all expected tables, all 9 `image_id_N` columns exist, migrations are idempotent, `set_page_image_slot` round-trips all 9 slots, and clears a slot back to `None` |
| `src/core/pdf.rs` | Per-template render smoke test | Zero tests in `pdf.rs`; the gridTwo bug shipped invisibly because nothing renders and inspects each of the 6+ templates | ✅ Done | One test per `TemplateId` (16 templates) asserting `page.get_xobject_ids().len()` matches the expected image count — `grid_two_renders_both_images` is a direct regression test for the bug that shipped |
| `src/core/develop.rs` + `src/core/pdf.rs` | Crop/rotate persists through zine export | Geometry (rotation/crop) is applied at Develop-save time and cached; no test proves the *exported PDF* reflects it | ✅ Done | `crop_and_rotate_persist_through_zine_render` reproduces the full `commands.rs` embed pipeline (apply_develop → JPEG encode → `RawImage` → `render_page`) and asserts the geometry-adjusted (not original) dimensions land in the rendered page |

## Frontend test infrastructure — required

Currently `frontend/` has no test runner at all (`package.json` has no `test`
script). This is the one item the user has called a hard requirement, not
optional cleanup.

**Decided approach:** Vitest + `@testing-library/react`, since Vitest reuses
the existing `vite.config.ts` with zero extra bundler config. Tauri IPC is
mocked one layer below `lib/ipc.ts` — `vi.mock('@tauri-apps/api/core')`
replaces `invoke` with a `vi.fn()` dispatched by command-name string, so the
real `ipc.ts` wrapper functions and real `store.ts` logic run under test
against canned responses, never a live Rust backend. This was chosen over
mocking `lib/ipc.ts` itself because it covers all ~30 exported wrappers from
one mock point instead of maintaining a parallel mock for each. True
end-to-end (driving the actual compiled app via `tauri-driver` + WebDriver)
remains a stretch item — it's the only way to test real IPC + SQLite + PDF
output together, but it needs a built binary per run and is much slower, so
it doesn't block the "must have" bar.

Vitest was pinned to the 3.x line (not the newer 4.x) because 4.x requires
Vite 6/7/8 and this project is still on Vite 5.4.21 — 3.x supports Vite 5
without forcing an unrelated build-tool upgrade.

| File | Task | One-liner | Status | Pending items |
|---|---|---|---|---|
| `frontend/package.json`, `frontend/vite.config.ts` | Vitest setup | Added `vitest@^3.2.7`, `@testing-library/react`, `@testing-library/jest-dom`, `jsdom`; `test`/`test:watch` scripts; `test` block in `vite.config.ts` (now imports `defineConfig` from `vitest/config`); `frontend/src/test/setup.ts` wires up jest-dom matchers | ✅ Done | `npm --prefix frontend run test` — verified against both `tsc --noEmit` and `npm run build` (production bundle unaffected) |
| `@tauri-apps/api/core` mock | IPC mock seam | `vi.mock('@tauri-apps/api/core')` replaces `invoke` with a command-name-dispatched `vi.fn()`; no changes needed to `ipc.ts` itself | ✅ Done | — |
| `frontend/src/store.ts` | Store unit tests | Zustand actions are pure enough to test without rendering anything | ✅ Done | 7 tests in `frontend/src/store.test.ts`: session load + focus, import-cancellation (no toast) vs. real failure (toast), cull-state filter re-derivation (both drop and keep cases), filter fallback to first-visible / clear-when-empty |
| `frontend/src/stages/*.tsx` | Import → Pick → Zine → Export smoke test | The one flow the user specifically wants covered | ✅ Done | `frontend/src/test/flows/importPickZineExport.test.ts` — Import/Pick driven through the real store (`useStore`), Zine/Export driven directly against the `ipc.*` wrappers in the same call sequence `ZineStage.tsx` uses; asserts the full backend call order and exact args. Scoped at the store/ipc level rather than full RTL rendering of the 1500-line `ZineStage.tsx` — that's deferred to the code-health refactor (splitting it into smaller components first makes it actually renderable in a test) or the stretch E2E item |
| — | (stretch) `tauri-driver` E2E | Real end-to-end against the compiled app | 🔲 Not started | Only pick up after `ZineStage.tsx`/`DevelopStage.tsx` are decomposed (code-health item) — testing a 1500-line component through either RTL or WebDriver is much easier post-refactor |

## Code health

| File | Task | One-liner | Status | Pending items |
|---|---|---|---|---|
| `src/commands.rs` | Split by domain | 2084 lines, every IPC handler in one file | ✅ Done | Split into `src/commands/{mod,folder,session,export,settings,caption,develop,zine,system}.rs` (largest is `zine.rs` at 481 lines). `mod.rs` holds the shared DTOs/helpers and re-exports every submodule with `pub use domain::*`, so `main.rs`'s `commands::fn_name` call sites needed zero changes. Verified: `cargo check`/`test` (47 unit + 3 integration, same count as before)/`fmt`/`clippy -D warnings` all clean |
| `frontend/src/stages/DevelopStage.tsx` | Decompose | 1567 lines | ✅ Done | Root slimmed to 238 lines. Split into `frontend/src/stages/develop/`: `geometry.ts` (crop math, no React), `CropOverlay.tsx`, `useNavigationKeys.ts`, `ExportPopover.tsx`, `OpenWithPopover.tsx`, `Toolbar.tsx`, `PreviewPanel.tsx`, `Slider.tsx`, `Section.tsx`, `AiSection.tsx`, `DevelopPanel.tsx`. The file was already internally factored into distinct components with clean prop boundaries, so this was pure code movement (no logic changes) — verified by `tsc`, `eslint`, `vitest`, and an unchanged production bundle size |
| `frontend/src/stages/ZineStage.tsx` | Decompose | 1095 lines | ✅ Done | Root slimmed to 441 lines. Split into `frontend/src/stages/zine/`: `TemplateIcon.tsx`, `SortablePage.tsx`, `PagePreview.tsx`, `ImagePicker.tsx`, `Inspector.tsx` (image slots, layout picker, caption override, AI suggestions). Same pure-movement approach and verification as DevelopStage |
| — | Visual verification (computer-use) | Clicked through both decomposed stages in the real app against a real photo session | ✅ Done | Develop: sliders, rotate, Save/dirty tracking, AI Auto-develop panel all render and respond correctly. Zine: template picker, `gridTwo` Left/Right slot assignment, and PDF export all verified end-to-end — opened the exported `zine.pdf` in Preview and confirmed page 1 shows both images side-by-side. Along the way, found and killed a stale release `.app` build (`target/release/bundle/macos/`) that macOS Launch Services kept resolving "shotlist" to instead of the live dev build — caused several false-alarm observations before being traced to its root cause; documented so it doesn't recur. Also found and separately flagged (not fixed here) a **pre-existing** bug, confirmed via `git show HEAD` to predate this session: toggling the Develop stage's Crop button discards unsaved slider/rotation edits, because `fetchPreview`'s `[cropMode]` dependency causes the image-switch effect to re-fire and reload saved params — see the spawned follow-up task |
| `frontend/` | ESLint | No lint config exists on the frontend at all | ✅ Done | Added `eslint.config.js` (flat config): `js`/`typescript-eslint` recommended sets, `react-hooks` (only the long-established `rules-of-hooks`/`exhaustive-deps` — v7's bundled `recommended` set pulls in React Compiler diagnostics that flagged ~24 pre-existing working patterns as errors, judged out of scope for a first lint pass), `react-refresh`. Fixed the 6 real findings it turned up: 2 stray `any` (one via widening `suggestions` state to `Array<SequenceSuggestion \| null>` + a proper type-guard filter, one via dropping a redundant `catch (e: any)`), an empty `catch {}`, and a `thumbnails` object recreated every render as a `useCallback` dep (wrapped in `useMemo`). Left the two intentional narrow-no-break-space (U+202F) unit-spacing characters alone — configured `skipTemplates: true` instead of stripping real typography. `npm --prefix frontend run lint` script added |

## Manual QA

| File | Task | One-liner | Status | Pending items |
|---|---|---|---|---|
| — | Full workflow walkthrough | No e2e harness exists yet; do this by hand once the above lands | 🔲 Not started | Real RAW + JPEG folder, 500+ images, through Import→Cull→Analyse→Develop→Caption→Zine→PDF export; confirm gridTwo fix, confirm exported PDF matches on-screen preview |

## Execution order

Agreed sequence — isolated/quick fixes first, gates cleaned before more code
lands on top of them, then coverage, then larger refactors:

1. Release packaging config (identifier + ATS domain)
2. gridTwo end-to-end fix
3. Quality gates (`cargo fmt`, clippy, fix the pre-commit config)
4. ~~API key → OS keychain~~ — **parked**, folded into a future AI-provider suite (Anthropic/OpenAI/Gemini/LM Studio together)
5. ~~Gemini provider: implement or drop~~ — **parked**, same future suite
6. Rust test coverage (DB, PDF-per-template, crop/rotate-through-export)
7. Frontend test infrastructure + import→pick→zine→export smoke test
8. Code health refactor (`commands.rs` split, stage decomposition, ESLint)
9. Code signing / notarization / distribution decision
10. Manual QA pass

Items 4 and 5 are deferred out of this sequence for now — revisit as a
dedicated AI-provider workstream later.
