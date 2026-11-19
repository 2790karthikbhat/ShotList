export function Slider({
  label,
  value,
  min,
  max,
  step = 1,
  decimals = 0,
  onChange,
  onReset,
}: {
  label: string;
  value: number;
  min: number;
  max: number;
  step?: number;
  decimals?: number;
  onChange: (v: number) => void;
  onReset: () => void;
}) {
  const pct = ((value - min) / (max - min)) * 100;
  const isDirty = value !== 0;
  const displayVal =
    decimals > 0
      ? value.toFixed(decimals)
      : value > 0
        ? `+${value}`
        : `${value}`;

  return (
    <div className="group">
      <div className="flex items-center justify-between mb-1">
        <button
          onDoubleClick={onReset}
          className="text-[11px] text-lo group-hover:text-mid transition-colors text-left"
          title="Double-click to reset"
        >
          {label}
          {isDirty && (
            <span className="ml-1 w-1 h-1 rounded-full bg-accent/70 inline-block align-middle" />
          )}
        </button>
        <span
          className={`text-[11px] font-mono tabular-nums w-10 text-right ${isDirty ? "text-accent" : "text-lo"}`}
        >
          {displayVal}
        </span>
      </div>
      <input
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={(e) => onChange(parseFloat(e.target.value))}
        className="w-full h-[5px] rounded-full appearance-none cursor-pointer"
        style={{
          background: `linear-gradient(to right, rgb(var(--color-accent-rgb)) ${pct}%, rgb(var(--color-surface-rgb)) ${pct}%)`,
          accentColor: "rgb(var(--color-accent-rgb))",
        }}
      />
    </div>
  );
}
