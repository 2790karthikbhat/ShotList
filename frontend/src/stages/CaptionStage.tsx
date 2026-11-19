// Phase 3 — Caption Stage
// Editorial metadata editor with Claude Vision AI generation.
//
// Layout:
//   Toolbar  (◉ Caption | session | nav | Save | Embed XMP)
//   FilterBar
//   ┌─────────────────────────────────┬──────────────────────────────────┐
//   │  Viewer (left, flex-1)          │  Caption panel (right, 360px)    │
//   │                                 │  [✦ Generate with Claude]        │
//   │                                 │  Title          ─ text input     │
//   │                                 │  Description    ─ textarea       │
//   │                                 │  Keywords       ─ tag chips      │
//   │                                 │  Location       ─ text input     │
//   │                                 │  Credit         ─ text input     │
//   └─────────────────────────────────┴──────────────────────────────────┘
//   Filmstrip

import { useEffect, useRef, useCallback, useState, KeyboardEvent } from "react";
import { listen } from "@tauri-apps/api/event";
import { useStore } from "../store";
import { Filmstrip } from "../components/Filmstrip";
import { MetadataPill } from "../components/MetadataPill";
import { FilterBar } from "../components/FilterBar";
import { StageToolbar } from "../components/StageToolbar";
import {
  IconTag,
  IconChevronLeft,
  IconChevronRight,
  IconSettings,
  IconSparkle,
  IconDownload,
} from "../components/icons";
import type { Caption, CaptionFieldEvent } from "../types";
import { DEFAULT_CAPTION } from "../types";
import * as ipc from "../lib/ipc";

interface BatchProgress {
  index: number;
  total: number;
  imageId: string;
}
interface BatchDone {
  imageId: string;
  caption: Caption;
}

// ── Keyboard handler ──────────────────────────────────────────────────────────

function useNavigationKeys() {
  const navigate = useStore((s) => s.navigate);
  const session = useStore((s) => s.session);

  const handleKey = useCallback(
    (e: KeyboardEvent) => {
      // Don't steal keypresses from text inputs / textareas.
      const tag = (e.target as HTMLElement).tagName;
      if (tag === "INPUT" || tag === "TEXTAREA") return;
      if (!session) return;
      switch (e.key) {
        case "ArrowRight":
        case "l":
        case "L":
          e.preventDefault();
          navigate("next");
          break;
        case "ArrowLeft":
        case "j":
        case "J":
          e.preventDefault();
          navigate("prev");
          break;
      }
    },
    [session, navigate],
  );

  useEffect(() => {
    window.addEventListener("keydown", handleKey as unknown as EventListener);
    return () =>
      window.removeEventListener(
        "keydown",
        handleKey as unknown as EventListener,
      );
  }, [handleKey]);
}

// ── Toolbar ───────────────────────────────────────────────────────────────────

function Toolbar({
  onSave,
  onEmbed,
  onGenerateAll,
  saving,
  embedding,
  batchGenerating,
  batchProgress,
  dirty,
}: {
  onSave: () => void;
  onEmbed: () => void;
  onGenerateAll: () => void;
  saving: boolean;
  embedding: boolean;
  batchGenerating: boolean;
  batchProgress: BatchProgress | null;
  dirty: boolean;
}) {
  const session = useStore((s) => s.session);
  const navigate = useStore((s) => s.navigate);
  const openSettings = useStore((s) => s.openSettings);

  return (
    <div className="flex flex-col flex-shrink-0">
      <StageToolbar
        icon={IconTag}
        stageLabel="Caption"
        sessionName={session?.name}
        right={
          session ? (
            <div className="flex items-center gap-2">
              <button
                onClick={onGenerateAll}
                disabled={batchGenerating}
                className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs text-mid
                           hover:text-hi hover:bg-surface transition-colors duration-100"
                title="Generate captions for all picks in one go (skips already-captioned images)"
              >
                <IconSparkle className="text-base" />
                {batchGenerating ? "Generating…" : "Caption All"}
              </button>
              <button
                onClick={onEmbed}
                disabled={embedding}
                className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs text-mid
                           hover:text-hi hover:bg-surface transition-colors duration-100"
                title="Write all captions to XMP sidecars (Lightroom / Capture One readable)"
              >
                <IconDownload className="text-base" />
                {embedding ? "Embedding…" : "Embed XMP"}
              </button>
              <button
                onClick={openSettings}
                className="w-[36px] h-[36px] flex items-center justify-center rounded-lg text-lo hover:text-mid hover:bg-surface text-2xl transition-colors"
              >
                <IconSettings />
              </button>
            </div>
          ) : undefined
        }
      >
        {session && (
          <>
            <FilterBar />
            <button
              onClick={() => navigate("prev")}
              className="w-[32px] h-[32px] flex items-center justify-center rounded-lg text-mid hover:text-hi hover:bg-surface transition-colors text-xl"
              title="Previous (← / J)"
            >
              <IconChevronLeft />
            </button>
            <button
              onClick={() => navigate("next")}
              className="w-[32px] h-[32px] flex items-center justify-center rounded-lg text-mid hover:text-hi hover:bg-surface transition-colors text-xl"
              title="Next (→ / L)"
            >
              <IconChevronRight />
            </button>

            {dirty && (
              <button
                onClick={onSave}
                disabled={saving}
                className="px-3 py-1.5 rounded-lg text-[11px] bg-accent text-black font-medium
                           hover:brightness-110 disabled:opacity-50 transition-all"
              >
                {saving ? "Saving…" : "Save"}
              </button>
            )}
          </>
        )}
      </StageToolbar>

      {/* Batch progress bar */}
      {batchGenerating && batchProgress && (
        <div className="vibrancy-panel border-b border-border px-4 py-2 flex items-center gap-3">
          <div className="flex-1 h-[5px] rounded-full bg-surface overflow-hidden">
            <div
              className="h-full bg-accent rounded-full transition-all duration-300"
              style={{
                width: `${((batchProgress.index + 1) / batchProgress.total) * 100}%`,
              }}
            />
          </div>
          <span className="text-[10px] text-lo shrink-0 tabular-nums">
            {batchProgress.index + 1} / {batchProgress.total}
          </span>
        </div>
      )}
    </div>
  );
}

// ── Viewer (left) ─────────────────────────────────────────────────────────────

function Viewer() {
  const session = useStore((s) => s.session);
  const currentId = useStore((s) => s.currentId);
  const images = useStore((s) => s.images);
  const viewerSrc = useStore((s) => s.viewerSrc);
  const viewerLoading = useStore((s) => s.viewerLoading);

  if (!session || !currentId) {
    return (
      <div className="flex-1 flex items-center justify-center bg-bg text-lo text-sm">
        Import a folder in the Cull stage first.
      </div>
    );
  }

  const img = images[currentId];
  if (!img) return null;

  return (
    <div className="flex-1 relative bg-bg overflow-hidden p-6">
      <div className="absolute inset-6 rounded-xl overflow-hidden shadow-panel bg-panel/40">
        {viewerSrc ? (
          <img
            src={viewerSrc}
            alt={img.filename}
            className="absolute inset-0 w-full h-full object-contain"
            draggable={false}
          />
        ) : (
          <div className="absolute inset-0 flex items-center justify-center">
            {viewerLoading && (
              <span className="text-lo text-xs tracking-widest animate-pulse">
                Loading…
              </span>
            )}
          </div>
        )}
      </div>
    </div>
  );
}

// ── Keyword chips editor ──────────────────────────────────────────────────────

function KeywordEditor({
  keywords,
  onChange,
}: {
  keywords: string[];
  onChange: (kw: string[]) => void;
}) {
  const [draft, setDraft] = useState("");

  const commit = () => {
    const cleaned = draft.trim().toLowerCase();
    if (cleaned && !keywords.includes(cleaned)) {
      onChange([...keywords, cleaned]);
    }
    setDraft("");
  };

  const handleKeyDown = (e: KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "Enter" || e.key === ",") {
      e.preventDefault();
      commit();
    } else if (e.key === "Backspace" && draft === "" && keywords.length > 0) {
      onChange(keywords.slice(0, -1));
    }
  };

  const remove = (kw: string) => onChange(keywords.filter((k) => k !== kw));

  return (
    <div
      className="min-h-[60px] rounded border border-border bg-bg/50 p-1.5 flex flex-wrap gap-1 cursor-text"
      onClick={() => document.getElementById("keyword-input")?.focus()}
    >
      {keywords.map((kw) => (
        <span
          key={kw}
          className="flex items-center gap-1 px-2 py-0.5 rounded-full bg-surface border border-border/60 text-[11px] text-mid"
        >
          {kw}
          <button
            onClick={(e) => {
              e.stopPropagation();
              remove(kw);
            }}
            className="text-lo hover:text-hi leading-none"
          >
            ×
          </button>
        </span>
      ))}
      <input
        id="keyword-input"
        value={draft}
        onChange={(e) => setDraft(e.target.value)}
        onKeyDown={handleKeyDown}
        onBlur={commit}
        placeholder={keywords.length === 0 ? "Type keywords, press Enter…" : ""}
        className="flex-1 min-w-[120px] bg-transparent text-[11px] text-hi placeholder:text-lo outline-none px-1"
      />
    </div>
  );
}

// ── Caption panel (right) ─────────────────────────────────────────────────────

function CaptionPanel({
  imageId,
  caption,
  setCaption,
  generating,
  onGenerate,
  dirty,
  saving,
  onSave,
}: {
  imageId: string | null;
  caption: Caption;
  setCaption: (c: Caption) => void;
  generating: boolean;
  onGenerate: () => void;
  dirty: boolean;
  saving: boolean;
  onSave: () => void;
}) {
  const set = (field: keyof Caption) => (value: string | string[]) =>
    setCaption({ ...caption, [field]: value });

  const fieldCount =
    [
      caption.title,
      caption.description,
      caption.location,
      caption.credit,
    ].filter(Boolean).length + (caption.keywords.length > 0 ? 1 : 0);

  return (
    <div className="flex flex-col h-full overflow-y-auto p-3.5 gap-3.5">
      {/* AI generate button — hero moment, not a thin outlined bar */}
      <div>
        <button
          onClick={onGenerate}
          disabled={!imageId || generating}
          className={`w-full py-3.5 rounded-xl text-[13px] font-medium transition-all flex items-center justify-center gap-2 ${
            generating
              ? "bg-surface text-lo cursor-not-allowed"
              : "bg-gradient-to-br from-accent/25 to-surface hover:from-accent/35 text-hi"
          }`}
        >
          {generating ? (
            <>
              <span className="w-2 h-2 rounded-full bg-accent animate-pulse" />
              Generating with AI…
            </>
          ) : (
            <>
              <IconSparkle className="text-accent text-lg" /> Generate caption with AI
            </>
          )}
        </button>

        {fieldCount > 0 && !generating && (
          <p className="text-[10px] text-lo text-center mt-1.5">
            {fieldCount} of 5 fields filled
          </p>
        )}
      </div>

      {/* Fields */}
      <div className="bg-surface rounded-xl px-3.5 py-4 space-y-5 flex-1">
        <Field label="Title" hint="5–10 words, newspaper style">
          <input
            type="text"
            value={caption.title}
            onChange={(e) => set("title")(e.target.value)}
            placeholder="Enter title…"
            className="input text-[12px]"
          />
        </Field>

        <Field
          label="Description"
          hint="2–3 sentences, journalistic present tense"
        >
          <textarea
            value={caption.description}
            onChange={(e) => set("description")(e.target.value)}
            placeholder="Describe what you see…"
            rows={4}
            className="input text-[12px] resize-none leading-relaxed"
          />
        </Field>

        <Field label="Keywords" hint="Enter, comma, or backspace to edit">
          <KeywordEditor
            keywords={caption.keywords}
            onChange={(kw) => set("keywords")(kw)}
          />
        </Field>

        <Field label="Location" hint="City, Country (leave blank if unknown)">
          <input
            type="text"
            value={caption.location}
            onChange={(e) => set("location")(e.target.value)}
            placeholder="e.g. Tokyo, Japan"
            className="input text-[12px]"
          />
        </Field>

        <Field label="Credit" hint="Photographer / agency name">
          <input
            type="text"
            value={caption.credit}
            onChange={(e) => set("credit")(e.target.value)}
            placeholder="Your name or agency"
            className="input text-[12px]"
          />
        </Field>
      </div>

      {/* Save button — shown whenever there are unsaved edits */}
      {dirty && (
        <button
          onClick={onSave}
          disabled={saving}
          className="w-full py-2.5 rounded-lg text-[11px] font-medium transition-all
                     bg-accent text-black hover:brightness-110 disabled:opacity-50"
        >
          {saving ? "Saving…" : "Save changes"}
        </button>
      )}

      {/* XMP info footer */}
      <p className="text-[10px] text-lo leading-relaxed">
        Captions are saved to the project database. Use{" "}
        <span className="text-mid inline-flex items-center gap-0.5">
          <IconDownload /> Embed XMP
        </span>{" "}
        in the toolbar to write them to sidecar files readable by
        Lightroom, Capture One, and Bridge.
      </p>
    </div>
  );
}

function Field({
  label,
  hint,
  children,
}: {
  label: string;
  hint?: string;
  children: React.ReactNode;
}) {
  return (
    <div className="space-y-1.5">
      <div>
        <p className="text-[10px] uppercase tracking-widest text-lo">{label}</p>
        {hint && <p className="text-[10px] text-lo/60 mt-0.5">{hint}</p>}
      </div>
      {children}
    </div>
  );
}

// ── CaptionStage root ─────────────────────────────────────────────────────────

export function CaptionStage() {
  useNavigationKeys();

  const currentId = useStore((s) => s.currentId);
  const images = useStore((s) => s.images);
  const visibleIds = useStore((s) => s.visibleIds);
  const showToast = useStore((s) => s.showToast);

  const [caption, setCaption] = useState<Caption>(DEFAULT_CAPTION);
  const [generating, setGenerating] = useState(false);
  const [saving, setSaving] = useState(false);
  const [embedding, setEmbedding] = useState(false);
  const [batchGenerating, setBatchGenerating] = useState(false);
  const [batchProgress, setBatchProgress] = useState<BatchProgress | null>(
    null,
  );

  // Track which imageId we loaded for (avoid stale saves on fast navigation).
  const loadedForRef = useRef<string | null>(null);
  const savedCaptionRef = useRef<Caption>(DEFAULT_CAPTION);

  const dirty =
    JSON.stringify(caption) !== JSON.stringify(savedCaptionRef.current);

  // Load caption whenever image changes.
  useEffect(() => {
    if (!currentId) return;
    loadedForRef.current = currentId;

    ipc
      .getCaption(currentId)
      .then((c) => {
        if (loadedForRef.current !== currentId) return;
        setCaption(c);
        savedCaptionRef.current = c;
        setGenerating(false);
      })
      .catch(() => {
        setCaption(DEFAULT_CAPTION);
        savedCaptionRef.current = DEFAULT_CAPTION;
      });
  }, [currentId]);

  // Listen for streaming caption-field events from Claude.
  useEffect(() => {
    if (!generating) return;

    const unlisten = listen<CaptionFieldEvent>("caption-field", (event) => {
      const { field, value } = event.payload;
      setCaption((prev) => {
        switch (field) {
          case "title":
          case "description":
          case "location":
          case "credit":
            return { ...prev, [field]: value as string };
          case "keywords":
            return { ...prev, keywords: value as string[] };
          default:
            return prev;
        }
      });
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [generating]);

  const handleGenerate = async () => {
    if (!currentId) return;
    setGenerating(true);
    setCaption(DEFAULT_CAPTION);
    savedCaptionRef.current = DEFAULT_CAPTION;

    try {
      const result = await ipc.generateCaption(currentId);
      if (loadedForRef.current === currentId) {
        setCaption(result);
        savedCaptionRef.current = result;
        showToast("Caption generated and saved", "success");
      }
    } catch (e) {
      showToast(String(e), "error");
    } finally {
      setGenerating(false);
    }
  };

  const handleGenerateAll = async () => {
    setBatchGenerating(true);
    setBatchProgress(null);

    // Progress events — update progress bar.
    const unProgress = await listen<BatchProgress>(
      "caption-batch-progress",
      (ev) => {
        setBatchProgress(ev.payload);
      },
    );

    // Done events — hydrate caption if we're on that image; update savedRef too.
    const unDone = await listen<BatchDone>("caption-batch-done", (ev) => {
      if (loadedForRef.current === ev.payload.imageId) {
        setCaption(ev.payload.caption);
        savedCaptionRef.current = ev.payload.caption;
      }
    });

    try {
      const summary = await ipc.generateAllCaptions(visibleIds, false);
      showToast(summary, "success");
    } catch (e) {
      showToast(String(e), "error");
    } finally {
      unProgress();
      unDone();
      setBatchGenerating(false);
      setBatchProgress(null);
      // Reload the current image's caption to ensure UI is fresh.
      if (currentId) {
        ipc
          .getCaption(currentId)
          .then((c) => {
            if (loadedForRef.current === currentId) {
              setCaption(c);
              savedCaptionRef.current = c;
            }
          })
          .catch(() => {});
      }
    }
  };

  const handleSave = async () => {
    if (!currentId) return;
    setSaving(true);
    try {
      await ipc.saveCaption(currentId, caption);
      savedCaptionRef.current = { ...caption };
      showToast("Caption saved", "success");
    } catch (e) {
      showToast(String(e), "error");
    } finally {
      setSaving(false);
    }
  };

  const handleEmbed = async () => {
    setEmbedding(true);
    try {
      const summary = await ipc.embedCaptions();
      showToast(summary, "success");
    } catch (e) {
      showToast(String(e), "error");
    } finally {
      setEmbedding(false);
    }
  };

  return (
    <div className="flex flex-col flex-1 overflow-hidden">
      <Toolbar
        onSave={handleSave}
        onEmbed={handleEmbed}
        onGenerateAll={handleGenerateAll}
        saving={saving}
        embedding={embedding}
        batchGenerating={batchGenerating}
        batchProgress={batchProgress}
        dirty={dirty}
      />

      {/* Middle: viewer (left) + caption panel (right) */}
      <div className="flex flex-1 overflow-hidden">
        <Viewer />

        {/* Right panel — fixed width, scrollable */}
        <div className="w-[360px] flex-shrink-0 border-l border-border vibrancy-panel overflow-hidden flex flex-col">
          <CaptionPanel
            imageId={currentId ?? null}
            caption={caption}
            setCaption={setCaption}
            generating={generating}
            onGenerate={handleGenerate}
            dirty={dirty}
            saving={saving}
            onSave={handleSave}
          />
        </div>
      </div>

      <Filmstrip
        trailing={
          currentId && images[currentId]
            ? <MetadataPill img={images[currentId]} showHints={false} />
            : undefined
        }
      />
    </div>
  );
}

