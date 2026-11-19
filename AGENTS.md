# AGENTS.md

Guide for AI coding agents (Claude Code, Codex, Cursor, Gemini CLI, Copilot and others) working in this repo.
Humans should start with [README.md](README.md).

ShotList is a desktop app for street and documentary photographers: import RAW/JPEG → cull → analyse →
develop → caption → print-ready zine PDF. Rust + Tauri 2 backend, React + TypeScript + Tailwind frontend,
SQLite storage. There is no server, no account and no telemetry.

## Setup (non-interactive)

```bash
npm run doctor   # check prerequisites only; prints the exact fix command for anything missing
npm run setup    # doctor + npm ci (frontend) + build frontend/dist + cargo build + enable the pre-commit hook
```

Both are `scripts/bootstrap.sh` and are safe to re-run. Exit codes: `0` ready, `1` missing prerequisite,
`2` a build step failed. A cold `cargo build` compiles ~500 crates and takes several minutes; don't
treat silence as a hang.

Prerequisites: Rust ≥ 1.88, Node ≥ 18, plus per-OS system libraries:

| OS      | Install                                                                                                                    |
| ------- | -------------------------------------------------------------------------------------------------------------------------- |
| macOS   | `xcode-select --install`                                                                                                   |
| Linux   | `sudo apt-get install -y build-essential pkg-config libssl-dev libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf` |
| Windows | MSVC build tools + WebView2 ([Tauri prerequisites](https://v2.tauri.app/start/prerequisites/))                            |

Everything else (SQLite, RAW decoding, ONNX inference via pure-Rust `tract`) is bundled through Cargo. No `libraw`, `vcpkg` or system SQLite.

Gotchas:

- The Tauri CLI lives in `frontend/node_modules`. Run it via `npm run tauri -- <args>` from the repo root.
  Bare `npx tauri` at the root prompts to download a package and blocks on stdin.
- `cargo build`/`cargo test` need `frontend/dist/` to exist (Tauri embeds it at compile time). `npm run setup`
  and `npm run check` build it if it's missing; otherwise run `npm --prefix frontend run build`.
- Linux builds need the system packages above even for `cargo check`.

## Commands

| Task                         | Command                                      |
| ---------------------------- | -------------------------------------------- |
| Run the app (hot reload)     | `npm run dev`                                |
| Release build + installers   | `npm run build`                              |
| **Everything CI runs**       | `npm run check`                              |
| Rust only                    | `cargo check` · `cargo test`                 |
| One Rust test                | `cargo test <name_substring>`                |
| Frontend tests               | `npm --prefix frontend run test`             |
| Frontend types / lint        | `npx --prefix frontend tsc --noEmit -p frontend` · `npm --prefix frontend run lint` |
| Benchmarks                   | `cargo bench`                                |

Before you call a change done, `npm run check` must pass. It runs `cargo fmt --check`,
`cargo clippy --all-targets --all-features -- -D warnings`, `cargo test`, ESLint, `tsc` and Vitest, the same as
`.github/workflows/ci.yml`. The pre-commit hook enforces fmt + clippy.

`npm run dev` opens a native window. In a headless or agent-only environment you can't see it: verify with
`npm run check`, and for UI work run the Vite dev server alone (`npm --prefix frontend run dev`,
port 5173). Note that IPC calls fail there because no Rust backend is attached.

## Layout

```
src/                    Rust backend (lib crate; main.rs is the Tauri entry)
├── main.rs             Window setup, command registration (generate_handler!), thumbnail emitter loop
├── commands/           All #[tauri::command] IPC handlers, one module per stage:
│                       session, folder, develop, caption, zine, export, settings, system
├── state.rs            AppState: session, DB, settings, cache dirs, thumbnail channel
├── settings.rs         AppSettings → <config_local_dir>/shotlist/settings.json
├── core/               Domain logic: image, session, import, exif, thumbnailer, analysis, develop,
│                       caption, zine, pdf, xmp, export, burst, watcher
├── ai/                 anthropic, openai, lmstudio (SSE streaming); develop/caption/sequence (streaming features);
│                       autocull, scoring, face_detector (ONNX UltraFace, local), local
└── db/                 rusqlite: mod.rs (open, sync_session), migrations.rs (V1–V6), queries.rs
tests/mvp_workflow.rs   Rust integration test
frontend/src/
├── App.tsx             Root, stage router, Settings modal
├── store.ts            Zustand store; subscribes to Tauri events
├── types.ts            TS mirrors of every Rust DTO
├── lib/ipc.ts          Typed invoke() wrappers, one per command
├── stages/             CullStage, AnalyseStage, DevelopStage, CaptionStage, ZineStage (+ develop/ and zine/ sub-folders)
├── components/         Shared UI; icons.tsx re-exports lucide-react
└── test/               Vitest setup + cross-cutting flow tests
docs/plans/             Feature status tracking (see below)
```

`docs/ARCHITECTURE.md` describes an earlier eframe prototype and is out of date. Trust the layout above.

## Adding an IPC command

1. Rust: `#[tauri::command] pub fn do_thing(...) -> Result<T, String>` in the right `src/commands/<stage>.rs`.
2. Register it in `src/main.rs` inside `tauri::generate_handler![...]`.
3. DTO structs: `#[derive(Serialize, Deserialize, Clone, Default)]` + `#[serde(rename_all = "camelCase")]`.
4. Mirror the DTO as an `interface` in `frontend/src/types.ts`; add a typed wrapper in `frontend/src/lib/ipc.ts`.
5. Streaming: Rust emits with `handle.emit("event-name", payload)`; frontend subscribes with `listen()` (see `store.ts`).

## Conventions

Rust

- `snake_case`; `camelCase` only on the wire via serde rename.
- `anyhow::Result` internally; `Result<_, String>` at the command boundary (Tauri requirement).
- `use crate::...` for internal modules. Shared state is behind `Mutex` (not `RwLock`).
- Don't add comments to new code unless asked.

TypeScript

- `camelCase`, `interface` for DTOs, named exports only, functional components only.
- Zustand for global state, `useState` for local form/UI state.
- Styling: Tailwind + `frontend/src/index.css` (theme tokens, `.input`, `.vibrancy-panel`). No CSS modules,
  no external UI kit.

Tests

- Rust unit tests sit next to the code (`#[cfg(test)]`), and the integration test is in `tests/`.
- Frontend tests mock Tauri with `vi.mock('@tauri-apps/api/core')`: a command-name-dispatched `vi.fn()`
  replaces `invoke()`, so real `ipc.ts`/`store.ts` logic runs against canned responses. Store tests live
  beside their source (`store.test.ts`), flow tests under `frontend/src/test/`.

## Data and privacy rules

- Never modify the user's original photos. Edits are non-destructive (DB rows + XMP sidecars), and exports go to
  `Picks/` or `Zine/` subfolders.
- Per-folder state lives in `<photo folder>/.shotlist.db`. Schema changes go through a new numbered migration in
  `src/db/migrations.rs`; never edit an existing migration.
- API keys live only in the local settings file. Never log them, put them in fixtures, or commit them.
- No network calls except the AI provider the user picked and the one-time face-model download.

## Feature tracking (`docs/plans/`)

`docs/plans/README.md` is the shipped-vs-in-flight reference. Each feature has `docs/plans/<feature>/00-overview.md`
with tables of `| File | Task | One-liner | Status | Pending items |`.
When your work meets a task's criteria, update its `Status` there, and update the feature row in
`docs/plans/INDEX.md` and `docs/plans/README.md` when the overall status changes. New features get a new folder and row.

Status legend: `🔲 Not started` · `🟡 In progress` · `✅ Done` · `⏳ Blocked/deferred`
