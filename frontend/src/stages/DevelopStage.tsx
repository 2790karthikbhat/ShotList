// Phase 2 — Develop Stage
// Non-destructive parametric darkroom with AI-assisted suggestions.
//
// Layout:
//   Toolbar (filter segmented control lives inline in the toolbar) ─────────
//   ┌─────────────────────────────────┬─────────────────────────────────┐
//   │  Preview (live)                 │  Develop panel (sliders + AI)   │
//   │  ← Before / After toggle →     │  Basic / Presence / WB / Effects │
//   └─────────────────────────────────┴─────────────────────────────────┘
//   Filmstrip ────────────────────────────────────────────────────────────

import { useEffect, useRef, useCallback, useState, useMemo } from "react";
import { useStore } from "../store";
import { Filmstrip } from "../components/Filmstrip";
import { MetadataPill } from "../components/MetadataPill";
import type { CropRect, DevelopParams } from "../types";
import { DEFAULT_DEVELOP_PARAMS } from "../types";
import * as ipc from "../lib/ipc";
import { DevelopPanel } from "./develop/DevelopPanel";
import { PreviewPanel } from "./develop/PreviewPanel";
import { Toolbar } from "./develop/Toolbar";
import { useNavigationKeys } from "./develop/useNavigationKeys";
import {
  CROP_PRESETS,
  centeredCropForRatio,
  orientedRatio,
} from "./develop/geometry";

export function DevelopStage() {
  useNavigationKeys();

  const currentId = useStore((s) => s.currentId);
  const images = useStore((s) => s.images);
  const showToast = useStore((s) => s.showToast);

  const [params, setParams] = useState<DevelopParams>(DEFAULT_DEVELOP_PARAMS);
  const [previewSrc, setPreview] = useState<string | null>(null);
  const [beforeSrc, setBeforeSrc] = useState<string | null>(null);
  const [previewLoading, setPreviewLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [showBefore, setShowBefore] = useState(false);
  const [cropMode, setCropMode] = useState(false);
  const [activePreset, setActivePreset] = useState("Free");
  const [naturalSize, setNaturalSize] = useState<{ w: number; h: number } | null>(
    null,
  );

  // Track which imageId we've loaded params for (avoid stale saves).
  const loadedForRef = useRef<string | null>(null);

  // Whether params differ from what's saved (or the default).
  const savedParamsRef = useRef<DevelopParams>(DEFAULT_DEVELOP_PARAMS);
  const dirty = useMemo(() => {
    return JSON.stringify(params) !== JSON.stringify(savedParamsRef.current);
  }, [params]);

  // Mirrors cropMode without being a fetchPreview dependency — toggling crop
  // must not change fetchPreview's identity, since that identity feeds the
  // image-switch effect below and would otherwise re-trigger it on every
  // crop toggle, discarding unsaved edits (see bug report).
  const cropModeRef = useRef(cropMode);
  useEffect(() => {
    cropModeRef.current = cropMode;
  }, [cropMode]);

  const fetchPreview = useCallback(async (id: string, p: DevelopParams) => {
    setPreviewLoading(true);
    try {
      // While actively cropping, show the full rotated frame (no crop baked
      // in yet) so the overlay has something uncropped to draw on top of.
      const requestParams = cropModeRef.current ? { ...p, crop: null } : p;
      const src = await ipc.getDevelopPreview(id, requestParams);
      if (loadedForRef.current === id) {
        setPreview(src);
      }
    } catch {
      // Thumbnail not cached yet — silently ignore.
    } finally {
      setPreviewLoading(false);
    }
  }, []);

  // Load saved params when image changes.
  const previewTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  // Set right before an image-switch fetches its first preview immediately —
  // the params-change effect below consumes and skips its usual debounce so
  // the very first frame for a newly-selected image isn't artificially
  // delayed by the 180ms slider-drag debounce.
  const skipNextDebounceRef = useRef(false);
  useEffect(() => {
    if (!currentId) return;
    loadedForRef.current = currentId;
    setPreview(null);
    setBeforeSrc(null);
    setShowBefore(false);
    setCropMode(false);
    setActivePreset("Free");
    setNaturalSize(null);
    if (previewTimerRef.current) clearTimeout(previewTimerRef.current);

    ipc.getImageSrc(currentId, true).then((src) => {
      if (loadedForRef.current === currentId) setBeforeSrc(src || null);
    }).catch(() => {});

    ipc
      .getDevelopParams(currentId)
      .then((p) => {
        if (loadedForRef.current !== currentId) return; // stale
        setParams(p);
        savedParamsRef.current = p;
        skipNextDebounceRef.current = true;
        fetchPreview(currentId, p);
      })
      .catch(() => {
        setParams(DEFAULT_DEVELOP_PARAMS);
        savedParamsRef.current = DEFAULT_DEVELOP_PARAMS;
        skipNextDebounceRef.current = true;
        fetchPreview(currentId, DEFAULT_DEVELOP_PARAMS);
      });
  }, [currentId, fetchPreview]);

  // Debounced preview fetch whenever params change from live edits (sliders,
  // crop drags) — skipped once right after an image switch, since that case
  // already triggered an immediate (non-debounced) fetch above.
  useEffect(() => {
    if (!currentId) return;
    if (showBefore) return; // no need to compute while viewing before
    if (skipNextDebounceRef.current) {
      skipNextDebounceRef.current = false;
      return;
    }

    if (previewTimerRef.current) clearTimeout(previewTimerRef.current);
    previewTimerRef.current = setTimeout(() => {
      fetchPreview(currentId, params);
    }, 180);

    return () => {
      if (previewTimerRef.current) clearTimeout(previewTimerRef.current);
    };
  }, [params, currentId, showBefore, fetchPreview]);

  // Setter helpers.
  const setParam = useCallback(
    (key: keyof DevelopParams, val: number | boolean) => {
      setParams((prev) => ({ ...prev, [key]: val }));
    },
    [],
  );

  const resetParam = useCallback((key: keyof DevelopParams) => {
    setParams((prev) => ({ ...prev, [key]: DEFAULT_DEVELOP_PARAMS[key] }));
  }, []);

  const setGeometry = useCallback((rotation: number, crop: CropRect | null) => {
    setParams((prev) => ({ ...prev, rotation, crop }));
  }, []);

  const handleCropChange = useCallback((rect: CropRect) => {
    setParams((prev) => ({ ...prev, crop: rect }));
  }, []);

  const handleSelectPreset = useCallback(
    (label: string) => {
      setActivePreset(label);
      const preset = CROP_PRESETS.find((p) => p.label === label);
      if (!preset || preset.ratio === null || !naturalSize) return; // Free: leave crop as-is
      const ratio = orientedRatio(preset.ratio, naturalSize.w, naturalSize.h);
      setParams((prev) => ({
        ...prev,
        crop: centeredCropForRatio(naturalSize.w, naturalSize.h, ratio),
      }));
    },
    [naturalSize],
  );

  const lockedRatio = useMemo(() => {
    if (activePreset === "Free" || !naturalSize) return null;
    const preset = CROP_PRESETS.find((p) => p.label === activePreset);
    if (!preset || preset.ratio === null) return null;
    return orientedRatio(preset.ratio, naturalSize.w, naturalSize.h);
  }, [activePreset, naturalSize]);

  // Save to DB.
  const handleSave = async () => {
    if (!currentId) return;
    setSaving(true);
    try {
      await ipc.saveDevelopParams(currentId, params);
      savedParamsRef.current = { ...params };
      showToast("Develop settings saved", "success");
    } catch (e) {
      showToast(String(e), "error");
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="flex flex-col flex-1 overflow-hidden">
      <Toolbar
        onSave={handleSave}
        saving={saving}
        dirty={dirty}
        currentId={currentId ?? null}
        currentParams={params}
      />

      {/* Middle: preview (left) + develop panel (right) */}
      <div className="flex flex-1 overflow-hidden">
        <PreviewPanel
          previewSrc={previewSrc}
          beforeSrc={beforeSrc}
          loading={previewLoading}
          showBefore={showBefore}
          setShowBefore={setShowBefore}
          cropMode={cropMode}
          crop={params.crop}
          lockedRatio={lockedRatio}
          onCropChange={handleCropChange}
          onNaturalSize={(w, h) => setNaturalSize({ w, h })}
        />

        {/* Right panel — fixed width, scrollable */}
        <div className="w-[340px] flex-shrink-0 border-l border-border vibrancy-panel overflow-hidden flex flex-col">
          <DevelopPanel
            params={params}
            setParam={setParam}
            resetParam={resetParam}
            imageId={currentId ?? null}
            cropMode={cropMode}
            setCropMode={setCropMode}
            activePreset={activePreset}
            onSelectPreset={handleSelectPreset}
            setGeometry={setGeometry}
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
