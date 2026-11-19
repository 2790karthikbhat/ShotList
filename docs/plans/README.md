# ShotList — Feature Implementation Status

Read this first to orient on what's shipped vs in-flight. Each row links to a
feature folder containing a `00-overview.md` with task-level detail.

| Feature | Status | Docs |
|---|---|---|
| Import (folder scan, EXIF, RAW preview) | ✅ Done | [import/00-overview.md](import/00-overview.md) |
| Cull (accept/reject, filtering, burst grouping) | ✅ Done | [cull/00-overview.md](cull/00-overview.md) |
| Analyse (histogram, Zone System, Laplacian heatmap) | ✅ Done | [analyse/00-overview.md](analyse/00-overview.md) |
| Develop (12-step apply pipeline, AI auto-develop) | ✅ Done | [develop/00-overview.md](develop/00-overview.md) |
| Caption (data model, AI streaming, batch generation) | ✅ Done | [caption/00-overview.md](caption/00-overview.md) |
| Zine (templates, PDF export, multi-image pages) | 🟡 In progress | [zine/00-overview.md](zine/00-overview.md) |
| AI Providers (Anthropic, OpenAI, LM Studio, Gemini) | 🟡 In progress | [ai-providers/00-overview.md](ai-providers/00-overview.md) |
| Rotate & Crop (Develop stage geometry) | ✅ Done | [rotate-crop/00-overview.md](rotate-crop/00-overview.md) |
| Navigation & Folder Browser (icons, nav rail, folder sidebar, Quick Look) | ✅ Done | [navigation/00-overview.md](navigation/00-overview.md) |
| Appearance (Light / Dark / System theme) | ✅ Done | [appearance/00-overview.md](appearance/00-overview.md) |
| Release Hardening (packaging, quality gates, security, test coverage) | 🔲 Not started | [release-hardening/00-overview.md](release-hardening/00-overview.md) |
| Public Release (license, CI, release builds, README) | 🟡 In progress | [public-release/00-overview.md](public-release/00-overview.md) |

Status legend: `🔲 Not started` · `🟡 In progress` · `✅ Done` · `⏳ Blocked/deferred`

When adding a new feature: create `docs/plans/<feature>/00-overview.md`, add a
row here, and keep [INDEX.md](INDEX.md) in sync.
