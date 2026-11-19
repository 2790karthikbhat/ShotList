> **Out of date:** this describes the early eframe prototype. For the current Tauri + React layout see [AGENTS.md](../AGENTS.md#layout).

# shotlist — Architecture

## Overview

```
shotlist/
├── src/
│   ├── main.rs          Binary entry point — logging + eframe window
│   ├── lib.rs           Library crate root (shared by binary + benches)
│   ├── core/            Domain model + filesystem I/O
│   │   ├── image.rs     ImageEntry, CullState, AiAssessment
│   │   ├── session.rs   Session, SessionFilter, SessionStats
│   │   ├── import.rs    Folder scan → Session
│   │   ├── export.rs    Copy picks into a Picks/ subfolder
│   │   ├── thumbnailer.rs  Off-thread thumbnail pipeline (rayon)
│   │   └── watcher.rs   Live folder watcher (notify)
│   ├── ai/              AI provider abstraction
│   │   ├── mod.rs       AiProvider trait + ProviderRegistry
│   │   ├── local.rs     Laplacian focus + ONNX face detection
│   │   ├── anthropic.rs Claude Vision implementation
│   │   ├── openai.rs    GPT-4o Vision implementation
│   │   └── scoring.rs   Scene grouping + composite score
│   ├── db/              SQLite persistence
│   │   ├── mod.rs       Connection setup + open()
│   │   ├── migrations.rs Embedded SQL migrations
│   │   └── queries.rs   Typed query functions
│   └── ui/              egui/eframe rendering
│       ├── mod.rs       CullApp struct + eframe::App impl
│       ├── theme.rs     Dark visual theme
│       ├── shortcuts.rs Keyboard shortcut handler
│       ├── panels.rs    Top bar + filmstrip
│       ├── viewer.rs    Main image viewer
│       └── settings.rs  Settings window
├── benches/
│   └── thumbnail_bench.rs  Criterion benchmarks
├── models/              ONNX model files (not in git — downloaded at runtime)
├── assets/              App icon, bundled fonts
├── docs/
│   └── ARCHITECTURE.md  This file
└── Cargo.toml
```

## Threading model

```
Main thread (egui)
│
├── Thumbnailer thread (std::thread)
│   └── rayon thread pool  — parallel RAW decode + resize
│
├── tokio runtime (for AI API calls)
│   ├── AnthropicProvider::assess_image (async)
│   └── OpenAiProvider::assess_image    (async)
│
└── notify watcher thread — filesystem events via crossbeam channel
```

All cross-thread communication uses `crossbeam` channels.
The UI thread never blocks — thumbnails and AI scores arrive via channel drain
in the `eframe::App::update()` loop.

## AI provider system

The `AiProvider` trait (in `src/ai/mod.rs`) is the single integration point.
Adding a new provider = implementing the trait + registering it in `main.rs`.

```rust
#[async_trait]
pub trait AiProvider: Send + Sync {
    fn name(&self) -> &'static str;
    async fn assess_image(&self, jpeg_bytes: &[u8]) -> Result<AiAssessment>;
}
```

API keys are stored in the OS keychain via the `keyring` crate.
No API keys are ever written to disk in plaintext.

## Database

One `.culld.db` SQLite file per project (sits alongside the source images).
Schema migrations are embedded in `src/db/migrations.rs` — new versions
run automatically on startup via a version table.

WAL journal mode is always enabled for better read concurrency.

## Status

The core app is complete: folder import (rfd picker), recursive scan of
JPEG/PNG/TIFF and camera RAW files, RAW embedded-preview extraction via the
pure-Rust `rawler` crate, keyboard culling, SQLite persistence with resume on
re-import, and pick export. ONNX inference and cloud AI are scaffolded but not
yet wired.

## Roadmap

- [ ] ONNX model download + face/eye detection
- [ ] Cloud AI scoring wired into the UI + batch cost-estimate dialog
- [ ] Scene grouping view
- [ ] Close-ups panel (face crop carousel)
- [ ] Lightroom export (XMP sidecar generation)
- [ ] Auto-update (via self_update crate)
