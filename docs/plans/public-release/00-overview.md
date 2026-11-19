# Public Release

Getting the repo ready to publish on GitHub: license, CI, tagged cross-platform
release builds, and a README written for people who have never heard of ShotList.
Pushing, tagging, and changing the repo's visibility are left to the maintainer.

## Repo hygiene

| File | Task | One-liner | Status | Pending items |
|---|---|---|---|---|
| (history) | Pre-publication audit | Scanned every commit for secrets, personal paths and names | ✅ Done | Old commits use a personal author email; one old `Cargo.toml` revision has it in `authors` |
| `LICENSE` | MIT license | Full MIT text, holder Karthik Bhat | ✅ Done | — |
| `Cargo.toml` | Package metadata | `authors` uses the GitHub noreply address; `repository` points at the real repo | ✅ Done | — |
| `.githooks/post-commit` | Untrack personal hook | Obsidian logging hook stays on disk (excluded locally) but leaves the repo | ✅ Done | — |

## CI & releases

| File | Task | One-liner | Status | Pending items |
|---|---|---|---|---|
| `.github/workflows/ci.yml` | CI on push/PR | fmt, clippy `-D warnings`, cargo test, ESLint, tsc, Vitest | ✅ Done | First run on GitHub |
| `.github/workflows/release.yml` | Release on `v*` tag | tauri-action builds a universal macOS .dmg, Windows .msi/.exe and Linux .AppImage/.deb into a draft release | ✅ Done | First tag; builds are unsigned |
| `assets/icons/` | Platform icons | Square icon set incl. `.ico`/`.icns`, needed by the Windows bundler | ✅ Done | Source art is 240×160, so a higher-res master would sharpen the large sizes |

## README

| File | Task | One-liner | Status | Pending items |
|---|---|---|---|---|
| `README.md` | Header for new visitors | Pitch, download link, install + Gatekeeper/SmartScreen workarounds, BYO-key note | ✅ Done | — |
| `docs/demo.gif` | Demo GIF | Cull → develop → zine recording | 🔲 Not started | Waiting on the maintainer's sample photo folder |
| — | Name check | "ShotList" vs existing photo apps | ✅ Done | Decision on whether to rename is the maintainer's |

## AI-agent readiness

| File | Task | One-liner | Status | Pending items |
|---|---|---|---|---|
| `AGENTS.md` | Cross-agent guide | Setup, commands, layout, conventions, privacy rules; `CLAUDE.md` imports it | ✅ Done | — |
| `scripts/bootstrap.sh` | Non-interactive setup | `npm run doctor` / `npm run setup`; prints the fix for each missing prerequisite; exit codes 0/1/2 | ✅ Done | Verified on macOS only; CI runs `--check` on Linux |
| `scripts/check.sh` | One verify command | `npm run check` runs exactly what CI runs | ✅ Done | — |
| `scripts/install.sh` | One-line installer | `curl … \| bash` installs the latest release on macOS/Linux | 🟡 In progress | Untested end to end until the first release is published |
| `llms.txt` | LLM doc index | Short machine-readable map of the docs | ✅ Done | — |
| `README.md` | Accuracy pass | Removed the OpenAI claim (not selectable in Settings), Rust 1.78 → 1.88, Linux deps, stale architecture tree | ✅ Done | — |
| — | Headless CLI / MCP server | Let agents cull, analyse and export without the GUI | 🔲 Not started | Needs a design decision |
