# AI Providers

| File | Task | One-liner | Status | Pending items |
|---|---|---|---|---|
| `src/ai/anthropic.rs` | Anthropic Claude | SSE-streaming Vision API, requires API key | ✅ Done | — |
| `src/ai/openai.rs` | OpenAI GPT-4o | SSE-streaming Vision API | ✅ Done | — |
| `src/ai/lmstudio.rs` | LM Studio | Local OpenAI-compatible HTTP + SSE | ✅ Done | — |
| `src/ai/local.rs` | Local provider glue | Shared local (non-API) provider plumbing | ✅ Done | — |
| Gemini (`provider-gemini` feature flag) | Google Gemini Vision | Feature flag exists in `Cargo.toml`; no `src/ai/gemini.rs` implementation yet | 🔲 Not started | Implement `gemini.rs`, wire into provider selection |
