# Zine

| File | Task | One-liner | Status | Pending items |
|---|---|---|---|---|
| `src/core/zine.rs` | Zine data model | `Zine`/`ZinePage`, `PaperSize`, `TemplateId` | ✅ Done | — |
| `src/core/pdf.rs` | PDF rendering | `printpdf` page rendering across 6 templates | 🟡 In progress | `gridTwo` only renders the primary image — see [release-hardening/00-overview.md](../release-hardening/00-overview.md) |
| `src/db/migrations.rs` | Multi-image pages | `zine_pages` schema + `image_id_2`…`image_id_9` columns | ✅ Done | — |
| `src/ai/sequence.rs` | AI sequencing | Streaming AI-suggested zine page sequence | ✅ Done | — |
| `frontend/src/stages/ZineStage.tsx` | Zine UI | Template editor, sequencing, export | 🟡 In progress | `gridTwo` inspector has no slot-2 control and the preview shows a placeholder — see [release-hardening/00-overview.md](../release-hardening/00-overview.md) |
