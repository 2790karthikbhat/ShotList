import { useEffect, useRef, useState } from "react";
import { useStore } from "../../store";
import type { CropRect } from "../../types";
import { CropOverlay } from "./CropOverlay";
import { FULL_FRAME_CROP, type ContentBox } from "./geometry";

export function PreviewPanel({
  previewSrc,
  beforeSrc,
  loading,
  showBefore,
  setShowBefore,
  cropMode,
  crop,
  lockedRatio,
  onCropChange,
  onNaturalSize,
}: {
  previewSrc: string | null;
  beforeSrc: string | null;
  loading: boolean;
  showBefore: boolean;
  setShowBefore: (v: boolean) => void;
  cropMode: boolean;
  crop: CropRect | null;
  lockedRatio: number | null;
  onCropChange: (rect: CropRect) => void;
  onNaturalSize: (w: number, h: number) => void;
}) {
  const session = useStore((s) => s.session);
  const currentId = useStore((s) => s.currentId);
  const images = useStore((s) => s.images);

  const containerRef = useRef<HTMLDivElement>(null);
  const [containerSize, setContainerSize] = useState({ w: 0, h: 0 });
  const [natural, setNatural] = useState<{ w: number; h: number } | null>(null);

  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;
    const observer = new ResizeObserver(([entry]) => {
      setContainerSize({ w: entry.contentRect.width, h: entry.contentRect.height });
    });
    observer.observe(el);
    return () => observer.disconnect();
  }, []);

  if (!session || !currentId) {
    return (
      <div className="flex-1 flex items-center justify-center bg-bg text-lo text-sm">
        Import a folder in the Cull stage first.
      </div>
    );
  }

  const img = images[currentId];
  if (!img) return null;

  // No fallback to the pristine image while previewSrc is loading — that's
  // exactly what caused a raw-then-developed flash on every image switch.
  const displaySrc = showBefore ? beforeSrc : previewSrc;

  let contentBox: ContentBox | null = null;
  if (natural && containerSize.w > 0 && containerSize.h > 0) {
    const scale = Math.min(
      containerSize.w / natural.w,
      containerSize.h / natural.h,
    );
    const width = natural.w * scale;
    const height = natural.h * scale;
    contentBox = {
      left: (containerSize.w - width) / 2,
      top: (containerSize.h - height) / 2,
      width,
      height,
    };
  }

  // The rounded/padded "card" framing lives on this OUTER div only — never
  // add padding to containerRef itself. containerRef is the ResizeObserver
  // target AND the positioning root the img/CropOverlay size against
  // (absolute inset-0), so any padding there would shrink its measured
  // contentRect while the children still bled into that padding (children
  // size to the padding box, not the content box), desyncing the crop
  // overlay's handles from the actual displayed image. Keeping containerRef
  // padding-free means its contentRect always equals the real image bounds.
  return (
    <div className="flex-1 relative bg-bg overflow-hidden p-6">
      <div ref={containerRef} className="absolute inset-6 rounded-xl overflow-hidden shadow-panel bg-panel/40">
      {displaySrc ? (
        <img
          src={displaySrc}
          alt={img.filename}
          className="absolute inset-0 w-full h-full object-contain"
          draggable={false}
          style={{ transition: "opacity 0.15s ease" }}
          onLoad={(e) => {
            const el = e.currentTarget;
            setNatural({ w: el.naturalWidth, h: el.naturalHeight });
            onNaturalSize(el.naturalWidth, el.naturalHeight);
          }}
        />
      ) : (
        <div className="absolute inset-0 flex items-center justify-center">
          {loading && (
            <span className="text-lo text-xs tracking-widest animate-pulse">
              Loading…
            </span>
          )}
        </div>
      )}

      {cropMode && contentBox && (
        <CropOverlay
          contentBox={contentBox}
          crop={crop ?? FULL_FRAME_CROP}
          lockedRatio={lockedRatio}
          onCommit={onCropChange}
        />
      )}

      {/* Processing overlay */}
      {loading && previewSrc && !cropMode && (
        <div className="absolute top-3 right-3 bg-black/50 rounded px-2 py-1 flex items-center gap-1.5">
          <span className="w-2 h-2 rounded-full bg-accent animate-pulse" />
          <span className="text-[10px] text-lo">Processing…</span>
        </div>
      )}

      {/* Before / After toggle */}
      {!cropMode && (
        <div className="absolute bottom-3 left-1/2 -translate-x-1/2 flex rounded overflow-hidden border border-border">
          <button
            onClick={() => setShowBefore(false)}
            className={`px-3 py-1 text-[11px] transition-colors ${
              !showBefore
                ? "bg-accent text-black font-medium"
                : "bg-panel/80 text-mid hover:text-hi"
            }`}
          >
            After
          </button>
          <button
            onClick={() => setShowBefore(true)}
            className={`px-3 py-1 text-[11px] transition-colors ${
              showBefore
                ? "bg-accent text-black font-medium"
                : "bg-panel/80 text-mid hover:text-hi"
            }`}
          >
            Before
          </button>
        </div>
      )}
      </div>
    </div>
  );
}
