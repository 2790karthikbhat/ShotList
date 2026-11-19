import { useStore } from "../../store";
import * as ipc from "../../lib/ipc";

const KNOWN_EDITORS = [
  { label: "System Default", app: undefined },
  // { label: "Affinity Photo 2", app: "Affinity Photo 2" },
  { label: "Affinity Photo", app: "Affinity Photo" },
  { label: "Lightroom Classic", app: "Adobe Lightroom Classic" },
  { label: "Lightroom", app: "Adobe Lightroom" },
  { label: "darktable", app: "darktable" },
  { label: "Capture One", app: "Capture One" },
  { label: "RawTherapee", app: "RawTherapee" },
] as const;

export function OpenWithPopover({
  imagePath,
  onClose,
}: {
  imagePath: string;
  onClose: () => void;
}) {
  const showToast = useStore((s) => s.showToast);

  const openIn = async (appName?: string) => {
    try {
      await ipc.openWithApp(imagePath, appName);
      onClose();
    } catch (e) {
      showToast(
        `Could not open${appName ? ` in ${appName}` : ""}: ${e}`,
        "error",
      );
    }
  };

  return (
    <div
      className="absolute top-full right-0 mt-2 z-50 w-[190px] vibrancy-panel border border-border rounded-xl shadow-panel py-1.5"
      onClick={(e) => e.stopPropagation()}
    >
      {KNOWN_EDITORS.map(({ label, app }) => (
        <button
          key={label}
          onClick={() => openIn(app)}
          className="w-full text-left px-3.5 py-1.5 text-[11px] text-mid hover:bg-white/5 hover:text-hi transition-colors"
        >
          {label}
        </button>
      ))}
    </div>
  );
}
