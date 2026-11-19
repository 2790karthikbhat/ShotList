import { useEffect, useRef, useCallback, useState } from "react";
import type { CropRect } from "../../types";
import {
  HANDLE_IDS,
  HANDLE_POSITION,
  applyDrag,
  normalizedToPx,
  pxToNormalized,
  type ContentBox,
  type HandleId,
  type PxRect,
} from "./geometry";

export function CropOverlay({
  contentBox,
  crop,
  lockedRatio,
  onCommit,
}: {
  contentBox: ContentBox;
  crop: CropRect;
  lockedRatio: number | null;
  onCommit: (rect: CropRect) => void;
}) {
  const [pxRect, setPxRect] = useState<PxRect>(() =>
    normalizedToPx(crop, contentBox),
  );
  const pxRectRef = useRef(pxRect);
  pxRectRef.current = pxRect;
  const dragRef = useRef<{ mode: "move" | HandleId; startRect: PxRect } | null>(
    null,
  );

  useEffect(() => {
    setPxRect(normalizedToPx(crop, contentBox));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [crop, contentBox.left, contentBox.top, contentBox.width, contentBox.height]);

  const startDrag = useCallback(
    (mode: "move" | HandleId) => (e: React.PointerEvent) => {
      e.stopPropagation();
      e.preventDefault();
      const startX = e.clientX;
      const startY = e.clientY;
      dragRef.current = { mode, startRect: pxRectRef.current };

      const handleMove = (ev: PointerEvent) => {
        const drag = dragRef.current;
        if (!drag) return;
        setPxRect(
          applyDrag(drag, ev.clientX - startX, ev.clientY - startY, contentBox, lockedRatio),
        );
      };
      const handleUp = () => {
        window.removeEventListener("pointermove", handleMove);
        window.removeEventListener("pointerup", handleUp);
        dragRef.current = null;
        onCommit(pxToNormalized(pxRectRef.current, contentBox));
      };

      window.addEventListener("pointermove", handleMove);
      window.addEventListener("pointerup", handleUp);
    },
    [contentBox, lockedRatio, onCommit],
  );

  return (
    <div
      className="absolute"
      style={{
        left: contentBox.left,
        top: contentBox.top,
        width: contentBox.width,
        height: contentBox.height,
      }}
    >
      <div
        className="absolute bg-black/60"
        style={{ left: 0, top: 0, right: 0, height: pxRect.y }}
      />
      <div
        className="absolute bg-black/60"
        style={{ left: 0, top: pxRect.y + pxRect.height, right: 0, bottom: 0 }}
      />
      <div
        className="absolute bg-black/60"
        style={{ left: 0, top: pxRect.y, width: pxRect.x, height: pxRect.height }}
      />
      <div
        className="absolute bg-black/60"
        style={{
          left: pxRect.x + pxRect.width,
          top: pxRect.y,
          right: 0,
          height: pxRect.height,
        }}
      />

      <div
        onPointerDown={startDrag("move")}
        className="absolute border-2 border-white/90 cursor-move"
        style={{
          left: pxRect.x,
          top: pxRect.y,
          width: pxRect.width,
          height: pxRect.height,
        }}
      >
        {HANDLE_IDS.map((id) => (
          <div
            key={id}
            onPointerDown={startDrag(id)}
            className={`absolute w-3 h-3 rounded-full bg-white border border-black/40 ${HANDLE_POSITION[id]}`}
          />
        ))}
      </div>
    </div>
  );
}
