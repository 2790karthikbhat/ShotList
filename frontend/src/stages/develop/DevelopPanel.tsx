import { IconRotateLeft, IconRotateRight } from "../../components/icons";
import type { CropRect, DevelopParams } from "../../types";
import { DEFAULT_DEVELOP_PARAMS } from "../../types";
import { AiSection } from "./AiSection";
import { CROP_PRESETS } from "./geometry";
import { Section } from "./Section";
import { Slider } from "./Slider";

const GEOMETRY_KEYS = new Set<keyof DevelopParams>(["rotation", "crop"]);

export function DevelopPanel({
  params,
  setParam,
  resetParam,
  imageId,
  cropMode,
  setCropMode,
  activePreset,
  onSelectPreset,
  setGeometry,
}: {
  params: DevelopParams;
  setParam: (key: keyof DevelopParams, val: number | boolean) => void;
  resetParam: (key: keyof DevelopParams) => void;
  imageId: string | null;
  cropMode: boolean;
  setCropMode: (v: boolean) => void;
  activePreset: string;
  onSelectPreset: (label: string) => void;
  setGeometry: (rotation: number, crop: CropRect | null) => void;
}) {
  const s = (key: keyof DevelopParams) => ({
    value: params[key] as number,
    onChange: (v: number) => setParam(key, v),
    onReset: () => resetParam(key),
  });

  const handleApplySuggestion = (param: string, value: number) => {
    const knownParam = param as keyof DevelopParams;
    if (knownParam in DEFAULT_DEVELOP_PARAMS) {
      setParam(knownParam, value);
    }
  };

  const handleRevertParams = (snapshot: DevelopParams) => {
    (Object.keys(snapshot) as (keyof DevelopParams)[])
      .filter((k) => !GEOMETRY_KEYS.has(k))
      .forEach((k) => setParam(k, snapshot[k] as number | boolean));
    setGeometry(snapshot.rotation, snapshot.crop);
  };

  return (
    <div className="flex flex-col h-full overflow-y-auto gap-3 p-3">
      {/* Reset all */}
      <div className="flex items-center justify-between px-1 flex-shrink-0">
        <span className="text-[10px] uppercase tracking-widest text-lo">
          Adjustments
        </span>
        <button
          onClick={() => {
            (Object.keys(DEFAULT_DEVELOP_PARAMS) as (keyof DevelopParams)[])
              .filter((k) => !GEOMETRY_KEYS.has(k))
              .forEach((k) =>
                setParam(k, DEFAULT_DEVELOP_PARAMS[k] as number | boolean),
              );
            setGeometry(0, null);
            onSelectPreset("Free");
          }}
          className="text-[10px] text-lo hover:text-mid transition-colors"
        >
          Reset all
        </button>
      </div>

      <AiSection
        imageId={imageId}
        currentParams={params}
        onApplySuggestion={handleApplySuggestion}
        onRevertParams={handleRevertParams}
      />

      <Section title="Geometry">
        <div className="flex items-center gap-2">
          <button
            onClick={() => setGeometry((params.rotation + 270) % 360, params.crop)}
            className="w-7 h-7 flex items-center justify-center rounded text-mid hover:text-hi hover:bg-surface transition-colors text-sm"
            title="Rotate left 90°"
          >
            <IconRotateLeft />
          </button>
          <button
            onClick={() => setGeometry((params.rotation + 90) % 360, params.crop)}
            className="w-7 h-7 flex items-center justify-center rounded text-mid hover:text-hi hover:bg-surface transition-colors text-sm"
            title="Rotate right 90°"
          >
            <IconRotateRight />
          </button>
          <div className="flex-1" />
          <button
            onClick={() => setCropMode(!cropMode)}
            className={`px-2.5 py-1 rounded text-[11px] font-medium border transition-all ${
              cropMode
                ? "bg-accent text-black border-accent"
                : "bg-surface border-border text-mid hover:border-accent hover:text-accent"
            }`}
          >
            Crop
          </button>
        </div>

        {cropMode && (
          <>
            <div className="flex flex-wrap gap-1">
              {CROP_PRESETS.map((preset) => (
                <button
                  key={preset.label}
                  onClick={() => onSelectPreset(preset.label)}
                  className={`px-2 py-0.5 rounded text-[10px] border transition-colors ${
                    activePreset === preset.label
                      ? "bg-accent text-black border-accent"
                      : "bg-surface border-border text-mid hover:border-accent hover:text-accent"
                  }`}
                >
                  {preset.label}
                </button>
              ))}
            </div>
            <button
              onClick={() => {
                onSelectPreset("Free");
                setGeometry(params.rotation, null);
                setCropMode(false);
              }}
              className="text-[10px] text-lo hover:text-mid transition-colors"
            >
              Reset crop
            </button>
          </>
        )}
      </Section>

      <Section title="Basic">
        <Slider
          label="Exposure"
          min={-5}
          max={5}
          step={0.05}
          decimals={2}
          {...s("exposure")}
        />
        <Slider label="Contrast" min={-100} max={100} {...s("contrast")} />
        <Slider label="Highlights" min={-100} max={100} {...s("highlights")} />
        <Slider label="Shadows" min={-100} max={100} {...s("shadows")} />
        <Slider label="Whites" min={-100} max={100} {...s("whites")} />
        <Slider label="Blacks" min={-100} max={100} {...s("blacks")} />
      </Section>

      <Section title="Presence">
        <Slider label="Clarity" min={-100} max={100} {...s("clarity")} />
        <Slider label="Vibrance" min={-100} max={100} {...s("vibrance")} />
        <Slider label="Saturation" min={-100} max={100} {...s("saturation")} />
      </Section>

      <Section title="White Balance">
        <Slider
          label="Temperature"
          min={-100}
          max={100}
          {...s("temperature")}
        />
        <Slider label="Tint" min={-100} max={100} {...s("tint")} />
      </Section>

      <Section title="Effects" defaultOpen={false}>
        <Slider label="Vignette" min={-100} max={100} {...s("vignette")} />
        <Slider label="Grain" min={0} max={100} {...s("grain")} />
      </Section>

      <Section title="B&W" defaultOpen={false}>
        <div className="flex items-center justify-between">
          <span className="text-[11px] text-lo">Black &amp; White</span>
          <button
            onClick={() => setParam("bwEnabled", !params.bwEnabled)}
            className={`relative w-9 h-5 rounded-full flex-shrink-0 transition-colors duration-200 ${
              params.bwEnabled ? "bg-accent" : "bg-surface"
            }`}
          >
            <span
              className="absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-white shadow transition-transform duration-200"
              style={{
                transform: params.bwEnabled
                  ? "translateX(16px)"
                  : "translateX(0)",
              }}
            />
          </button>
        </div>
        {params.bwEnabled && (
          <>
            <p className="text-[10px] text-lo">
              Channel luminosity mix (double-click to reset)
            </p>
            <Slider label="Red" min={-100} max={100} {...s("bwRed")} />
            <Slider label="Orange" min={-100} max={100} {...s("bwOrange")} />
            <Slider label="Yellow" min={-100} max={100} {...s("bwYellow")} />
            <Slider label="Green" min={-100} max={100} {...s("bwGreen")} />
            <Slider label="Aqua" min={-100} max={100} {...s("bwAqua")} />
            <Slider label="Blue" min={-100} max={100} {...s("bwBlue")} />
          </>
        )}
      </Section>
    </div>
  );
}
