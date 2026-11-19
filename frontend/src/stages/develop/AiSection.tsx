import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { useStore } from "../../store";
import { IconSparkle, IconUndo, IconCheck } from "../../components/icons";
import type { DevelopParams, DevelopSuggestion } from "../../types";
import * as ipc from "../../lib/ipc";

const PARAM_LABELS: Record<string, string> = {
  exposure: "Exposure",
  contrast: "Contrast",
  highlights: "Highlights",
  shadows: "Shadows",
  whites: "Whites",
  blacks: "Blacks",
  clarity: "Clarity",
  vibrance: "Vibrance",
  saturation: "Saturation",
  temperature: "Temperature",
  tint: "Tint",
  vignette: "Vignette",
  grain: "Grain",
};

export function AiSection({
  imageId,
  currentParams,
  onApplySuggestion,
  onRevertParams,
}: {
  imageId: string | null;
  currentParams: DevelopParams;
  onApplySuggestion: (param: string, value: number) => void;
  onRevertParams: (snapshot: DevelopParams) => void;
}) {
  const showToast = useStore((s) => s.showToast);
  const [running, setRunning] = useState(false);
  const [done, setDone] = useState(false);
  const [suggestions, setSuggestions] = useState<DevelopSuggestion[]>([]);
  const [applied, setApplied] = useState<Set<number>>(new Set());
  const [preApplySnapshot, setPreApplySnapshot] =
    useState<DevelopParams | null>(null);

  // Listen for streaming suggestions.
  useEffect(() => {
    if (!running) return;

    const unlisten = listen<DevelopSuggestion>(
      "develop-suggestion",
      (event) => {
        setSuggestions((prev) => [...prev, event.payload]);
      },
    );

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [running]);

  const start = async () => {
    if (!imageId) return;
    setSuggestions([]);
    setApplied(new Set());
    setPreApplySnapshot(null);
    setDone(false);
    setRunning(true);

    try {
      await ipc.autoDevelop(imageId);
      setDone(true);
    } catch (e) {
      showToast(String(e), "error");
    } finally {
      setRunning(false);
    }
  };

  const apply = (idx: number, s: DevelopSuggestion) => {
    if (preApplySnapshot === null) setPreApplySnapshot({ ...currentParams });
    onApplySuggestion(s.param, s.value);
    setApplied((prev) => new Set([...prev, idx]));
  };

  const applyAll = () => {
    if (preApplySnapshot === null) setPreApplySnapshot({ ...currentParams });
    suggestions.forEach((s, i) => {
      if (!applied.has(i)) onApplySuggestion(s.param, s.value);
    });
    setApplied(new Set(suggestions.map((_, i) => i)));
  };

  const undoApply = () => {
    if (!preApplySnapshot) return;
    onRevertParams(preApplySnapshot);
    setPreApplySnapshot(null);
    setApplied(new Set());
  };

  const showUtilityRow =
    (preApplySnapshot && applied.size > 0) ||
    (done && suggestions.length > 0 && applied.size < suggestions.length);

  return (
    <div className="flex-shrink-0 space-y-2">
      {/* Utility row — Undo / Apply All, only shown once there's something to act on */}
      {showUtilityRow && (
        <div className="flex items-center justify-end gap-3 px-1">
          {preApplySnapshot && applied.size > 0 && (
            <button
              onClick={undoApply}
              className="text-[10px] text-lo hover:text-mid transition-colors"
              title="Revert all AI-applied changes"
            >
              <span className="inline-flex items-center gap-0.5"><IconUndo /> Undo</span>
            </button>
          )}
          {done &&
            suggestions.length > 0 &&
            applied.size < suggestions.length && (
              <button
                onClick={applyAll}
                className="text-[10px] text-accent hover:text-hi transition-colors"
              >
                Apply All
              </button>
            )}
        </div>
      )}

      {/* Hero button — matches Caption's "Generate with AI" treatment */}
      <button
        onClick={start}
        disabled={!imageId || running}
        className={`w-full py-3.5 rounded-xl text-[13px] font-medium transition-all flex items-center justify-center gap-2 ${
          running
            ? "bg-surface text-lo cursor-not-allowed"
            : "bg-gradient-to-br from-accent/25 to-surface hover:from-accent/35 text-hi"
        }`}
      >
        {running ? (
          <>
            <span className="w-2 h-2 rounded-full bg-accent animate-pulse" />
            Analysing with AI…
          </>
        ) : (
          <>
            <IconSparkle className="text-accent text-lg" /> Auto-develop with AI
          </>
        )}
      </button>

      {suggestions.length > 0 && (
        <div className="bg-surface rounded-xl p-2.5 space-y-2">
          {suggestions.map((s, i) => (
            <div
              key={i}
              className={`rounded-lg border px-2.5 py-2 transition-all ${
                applied.has(i)
                  ? "border-accent/30 bg-accent/5"
                  : "border-border bg-surface/50 hover:border-border/80"
              }`}
            >
              <div className="flex items-start justify-between gap-2">
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2">
                    <span className="text-[11px] text-mid font-medium">
                      {PARAM_LABELS[s.param] ?? s.param}
                    </span>
                    <span
                      className={`text-[11px] font-mono tabular-nums ${
                        applied.has(i) ? "text-accent" : "text-hi"
                      }`}
                    >
                      {s.value > 0 ? `+${s.value}` : s.value}
                    </span>
                  </div>
                  <p className="text-[10px] text-lo mt-0.5 leading-relaxed">
                    {s.reason}
                  </p>
                </div>
                {!applied.has(i) ? (
                  <button
                    onClick={() => apply(i, s)}
                    className="flex-shrink-0 px-2 py-0.5 rounded text-[10px] bg-surface border border-border hover:border-accent hover:text-accent transition-colors text-mid"
                  >
                    Apply
                  </button>
                ) : (
                  <span className="flex-shrink-0 text-accent">
                    <IconCheck />
                  </span>
                )}
              </div>
            </div>
          ))}

          {running && (
            <div className="flex items-center gap-2 py-1">
              <span className="w-1.5 h-1.5 rounded-full bg-accent animate-pulse" />
              <span className="text-[10px] text-lo">Streaming…</span>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
