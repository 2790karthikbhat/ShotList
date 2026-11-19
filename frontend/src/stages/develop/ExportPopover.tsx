import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { useStore } from "../../store";
import { IconDownload } from "../../components/icons";
import type { DevelopParams, ExportProgress } from "../../types";
import * as ipc from "../../lib/ipc";

export function ExportPopover({
  currentId,
  currentParams,
  onClose,
}: {
  currentId: string | null;
  currentParams: DevelopParams;
  onClose: () => void;
}) {
  const session = useStore((s) => s.session);
  const showToast = useStore((s) => s.showToast);

  const [quality, setQuality] = useState(92);
  const [exporting, setExporting] = useState(false);
  const [batchProgress, setBatchProgress] = useState<ExportProgress | null>(
    null,
  );

  const picksCount = session
    ? Object.values(useStore.getState().images).filter(
        (i) => i.cullState === "pick",
      ).length
    : 0;

  // Listen for batch progress events
  useEffect(() => {
    if (!exporting) return;
    const unlisten = listen<ExportProgress>("export-progress", (e) => {
      setBatchProgress(e.payload);
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [exporting]);

  const exportCurrent = async () => {
    if (!currentId) return;
    setExporting(true);
    try {
      const msg = await ipc.exportDeveloped(currentId, currentParams, quality);
      showToast(msg, "success");
      onClose();
    } catch (e) {
      showToast(String(e), "error");
    } finally {
      setExporting(false);
    }
  };

  const exportAll = async () => {
    setExporting(true);
    setBatchProgress(null);
    try {
      const msg = await ipc.exportAllPicks(quality);
      showToast(msg, "success");
      onClose();
    } catch (e) {
      showToast(String(e), "error");
    } finally {
      setExporting(false);
      setBatchProgress(null);
    }
  };

  return (
    <div
      className="absolute top-full right-0 mt-2 z-50 w-[260px] vibrancy-panel border border-border rounded-xl shadow-panel p-3.5 space-y-4"
      onClick={(e) => e.stopPropagation()}
    >
      {/* Quality */}
      <div>
        <div className="flex items-center justify-between mb-1.5">
          <span className="text-[10px] uppercase tracking-widest text-lo">
            JPEG Quality
          </span>
          <span className="text-[11px] font-mono text-accent">{quality}</span>
        </div>
        <input
          type="range"
          min={60}
          max={100}
          step={1}
          value={quality}
          onChange={(e) => setQuality(+e.target.value)}
          className="w-full h-[5px] rounded-full appearance-none cursor-pointer"
          style={{
            background: `linear-gradient(to right, rgb(var(--color-accent-rgb)) ${((quality - 60) / 40) * 100}%, rgb(var(--color-surface-rgb)) ${((quality - 60) / 40) * 100}%)`,
            accentColor: "rgb(var(--color-accent-rgb))",
          }}
        />
        <div className="flex justify-between mt-0.5">
          <span className="text-[10px] text-lo">60</span>
          <span className="text-[10px] text-lo">100</span>
        </div>
      </div>

      {/* Batch progress bar */}
      {batchProgress && (
        <div>
          <div className="h-1 rounded-full bg-surface overflow-hidden">
            <div
              className="h-full bg-accent rounded-full transition-all duration-300"
              style={{
                width: `${(batchProgress.done / batchProgress.total) * 100}%`,
              }}
            />
          </div>
          <p className="text-[10px] text-lo mt-1 truncate">
            {batchProgress.done}/{batchProgress.total} —{" "}
            {batchProgress.filename}
          </p>
        </div>
      )}

      {/* Export buttons */}
      <div className="space-y-2">
        <button
          onClick={exportCurrent}
          disabled={!currentId || exporting}
          className="w-full py-2.5 rounded-lg text-[11px] font-medium bg-surface border border-border
                     text-mid hover:border-accent hover:text-accent disabled:opacity-40 transition-all"
        >
          <span className="inline-flex items-center gap-1"><IconDownload /> This image</span>
        </button>
        <button
          onClick={exportAll}
          disabled={exporting || picksCount === 0}
          className="w-full py-2.5 rounded-lg text-[11px] font-medium bg-accent text-black
                     hover:brightness-110 disabled:opacity-40 transition-all"
        >
          {exporting && batchProgress ? (
            `Exporting ${batchProgress.done}/${batchProgress.total}…`
          ) : (
            <span className="inline-flex items-center gap-1">
              <IconDownload /> All picks{picksCount > 0 ? ` (${picksCount})` : ""}
            </span>
          )}
        </button>
      </div>

      <p className="text-[10px] text-lo leading-relaxed">
        Exports to <span className="text-mid">Developed/</span> folder next to
        your source images. Develop params + captions embedded in XMP sidecar.
      </p>
    </div>
  );
}
