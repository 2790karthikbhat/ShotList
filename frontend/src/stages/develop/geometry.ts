// Crop geometry — pure math shared between CropOverlay, PreviewPanel, and
// DevelopStage's preset/rotation handlers. No React here.

import type { CropRect } from "../../types";

export const CROP_PRESETS: { label: string; ratio: number | null }[] = [
  { label: "Free", ratio: null },
  { label: "1:1", ratio: 1 },
  { label: "4:3", ratio: 4 / 3 },
  { label: "3:2", ratio: 3 / 2 },
  { label: "16:9", ratio: 16 / 9 },
  { label: "4:1", ratio: 4 },
];

/** Flip a landscape-form ratio (w/h) to portrait when the image is taller than wide. */
export function orientedRatio(baseRatio: number, imgW: number, imgH: number): number {
  return imgH > imgW ? 1 / baseRatio : baseRatio;
}

/** Largest centered rect of the given w/h ratio that fits inside the frame. */
export function centeredCropForRatio(imgW: number, imgH: number, ratio: number): CropRect {
  const imgRatio = imgW / imgH;
  const w = ratio > imgRatio ? imgW : imgH * ratio;
  const h = ratio > imgRatio ? imgW / ratio : imgH;
  return {
    x: (imgW - w) / 2 / imgW,
    y: (imgH - h) / 2 / imgH,
    width: w / imgW,
    height: h / imgH,
  };
}

export const FULL_FRAME_CROP: CropRect = { x: 0.1, y: 0.1, width: 0.8, height: 0.8 };

// ── Crop overlay (drag/resize in plain CSS, no canvas) ─────────────────────────

export interface PxRect {
  x: number;
  y: number;
  width: number;
  height: number;
}
export interface ContentBox {
  left: number;
  top: number;
  width: number;
  height: number;
}

export type HandleId = "nw" | "n" | "ne" | "e" | "se" | "s" | "sw" | "w";
export const HANDLE_IDS: HandleId[] = ["nw", "n", "ne", "e", "se", "s", "sw", "w"];
export const HANDLE_EDGES: Record<
  HandleId,
  { top?: boolean; bottom?: boolean; left?: boolean; right?: boolean }
> = {
  nw: { top: true, left: true },
  n: { top: true },
  ne: { top: true, right: true },
  e: { right: true },
  se: { bottom: true, right: true },
  s: { bottom: true },
  sw: { bottom: true, left: true },
  w: { left: true },
};
export const HANDLE_POSITION: Record<HandleId, string> = {
  nw: "-top-1.5 -left-1.5 cursor-nwse-resize",
  n: "-top-1.5 left-1/2 -translate-x-1/2 cursor-ns-resize",
  ne: "-top-1.5 -right-1.5 cursor-nesw-resize",
  e: "top-1/2 -right-1.5 -translate-y-1/2 cursor-ew-resize",
  se: "-bottom-1.5 -right-1.5 cursor-nwse-resize",
  s: "-bottom-1.5 left-1/2 -translate-x-1/2 cursor-ns-resize",
  sw: "-bottom-1.5 -left-1.5 cursor-nesw-resize",
  w: "top-1/2 -left-1.5 -translate-y-1/2 cursor-ew-resize",
};

export const MIN_CROP_PX = 24;

export function clamp(v: number, min: number, max: number): number {
  return Math.min(Math.max(v, min), max);
}

export function normalizedToPx(crop: CropRect, box: ContentBox): PxRect {
  return {
    x: crop.x * box.width,
    y: crop.y * box.height,
    width: crop.width * box.width,
    height: crop.height * box.height,
  };
}

export function pxToNormalized(rect: PxRect, box: ContentBox): CropRect {
  return {
    x: rect.x / box.width,
    y: rect.y / box.height,
    width: rect.width / box.width,
    height: rect.height / box.height,
  };
}

/** Recompute the dragged rect from the drag's starting rect + pointer delta. */
export function applyDrag(
  drag: { mode: "move" | HandleId; startRect: PxRect },
  dx: number,
  dy: number,
  box: ContentBox,
  lockedRatio: number | null,
): PxRect {
  const { mode, startRect } = drag;
  const maxW = box.width;
  const maxH = box.height;

  if (mode === "move") {
    return {
      ...startRect,
      x: clamp(startRect.x + dx, 0, maxW - startRect.width),
      y: clamp(startRect.y + dy, 0, maxH - startRect.height),
    };
  }

  const edges = HANDLE_EDGES[mode];
  let { x, y, width, height } = startRect;

  if (edges.left) {
    const right = startRect.x + startRect.width;
    x = clamp(startRect.x + dx, 0, right - MIN_CROP_PX);
    width = right - x;
  }
  if (edges.right) {
    width = clamp(startRect.width + dx, MIN_CROP_PX, maxW - x);
  }
  if (edges.top) {
    const bottom = startRect.y + startRect.height;
    y = clamp(startRect.y + dy, 0, bottom - MIN_CROP_PX);
    height = bottom - y;
  }
  if (edges.bottom) {
    height = clamp(startRect.height + dy, MIN_CROP_PX, maxH - y);
  }

  if (lockedRatio) {
    if (edges.left || edges.right) {
      const newHeight = clamp(width / lockedRatio, MIN_CROP_PX, maxH);
      if (edges.top) {
        const bottom = startRect.y + startRect.height;
        y = clamp(bottom - newHeight, 0, maxH - MIN_CROP_PX);
        height = bottom - y;
      } else if (edges.bottom) {
        height = clamp(newHeight, MIN_CROP_PX, maxH - y);
      } else {
        const cy = startRect.y + startRect.height / 2;
        height = newHeight;
        y = clamp(cy - height / 2, 0, maxH - height);
      }
    } else if (edges.top || edges.bottom) {
      const newWidth = clamp(height * lockedRatio, MIN_CROP_PX, maxW);
      if (edges.left) {
        const right = startRect.x + startRect.width;
        x = clamp(right - newWidth, 0, maxW - MIN_CROP_PX);
        width = right - x;
      } else if (edges.right) {
        width = clamp(newWidth, MIN_CROP_PX, maxW - x);
      } else {
        const cx = startRect.x + startRect.width / 2;
        width = newWidth;
        x = clamp(cx - width / 2, 0, maxW - width);
      }
    }
  }

  return { x, y, width, height };
}
