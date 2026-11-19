import { useState } from "react";
import { useStore } from "../../store";
import { StageToolbar } from "../../components/StageToolbar";
import { FilterBar } from "../../components/FilterBar";
import {
  IconSliders,
  IconChevronLeft,
  IconChevronRight,
  IconSettings,
  IconDownload,
  IconExternalLink,
} from "../../components/icons";
import type { DevelopParams } from "../../types";
import { ExportPopover } from "./ExportPopover";
import { OpenWithPopover } from "./OpenWithPopover";

export function Toolbar({
  onSave,
  saving,
  dirty,
  currentId,
  currentParams,
}: {
  onSave: () => void;
  saving: boolean;
  dirty: boolean;
  currentId: string | null;
  currentParams: DevelopParams;
}) {
  const session = useStore((s) => s.session);
  const images = useStore((s) => s.images);
  const navigate = useStore((s) => s.navigate);
  const openSettings = useStore((s) => s.openSettings);
  const [exportOpen, setExportOpen] = useState(false);
  const [openWithOpen, setOpenWithOpen] = useState(false);
  const currentPath = currentId ? (images[currentId]?.path ?? null) : null;

  return (
    <StageToolbar
      icon={IconSliders}
      stageLabel="Develop"
      sessionName={session?.name}
      right={
        <div className="flex items-center gap-2">
          {/* Open With button + popover */}
          {session && currentPath && (
            <div className="relative">
              <button
                onClick={() => {
                  setOpenWithOpen((o) => !o);
                  setExportOpen(false);
                }}
                className={`px-3 py-1.5 rounded-lg text-[11px] font-medium border transition-all ${
                  openWithOpen
                    ? "bg-accent text-black border-accent"
                    : "bg-surface border-border text-mid hover:border-accent hover:text-accent"
                }`}
                title="Open current image in an external editor"
              >
                <span className="inline-flex items-center gap-1.5"><IconExternalLink className="text-base" /> Open In…</span>
              </button>
              {openWithOpen && (
                <>
                  <div
                    className="fixed inset-0 z-40"
                    onClick={() => setOpenWithOpen(false)}
                  />
                  <OpenWithPopover
                    imagePath={currentPath}
                    onClose={() => setOpenWithOpen(false)}
                  />
                </>
              )}
            </div>
          )}

          {/* Export button + popover */}
          {session && (
            <div className="relative">
              <button
                onClick={() => {
                  setExportOpen((o) => !o);
                  setOpenWithOpen(false);
                }}
                className={`px-3 py-1.5 rounded-lg text-[11px] font-medium border transition-all ${
                  exportOpen
                    ? "bg-accent text-black border-accent"
                    : "bg-surface border-border text-mid hover:border-accent hover:text-accent"
                }`}
              >
                <span className="inline-flex items-center gap-1.5"><IconDownload className="text-base" /> Export</span>
              </button>
              {exportOpen && (
                <>
                  {/* Click-away backdrop */}
                  <div
                    className="fixed inset-0 z-40"
                    onClick={() => setExportOpen(false)}
                  />
                  <ExportPopover
                    currentId={currentId}
                    currentParams={currentParams}
                    onClose={() => setExportOpen(false)}
                  />
                </>
              )}
            </div>
          )}

          <button
            onClick={openSettings}
            className="w-[36px] h-[36px] flex items-center justify-center rounded-lg text-lo hover:text-mid hover:bg-surface text-2xl transition-colors"
          >
            <IconSettings />
          </button>
        </div>
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
  );
}
