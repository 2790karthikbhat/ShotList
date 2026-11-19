// Shared toolbar shell used by every stage — replaces five near-identical
// hand-rolled toolbars (icon + stage label + session name + traffic-light
// clearance + settings gear repeated in each stage file). Each stage passes
// its own control cluster as children; `right` overrides the trailing
// settings-gear slot when a stage needs something else there.

import { useStore } from '../store'
import { IconSettings } from './icons'

type IconComponent = (props: React.SVGProps<SVGSVGElement>) => JSX.Element

interface StageToolbarProps {
  icon: IconComponent
  stageLabel: string
  sessionName?: string | null
  children?: React.ReactNode
  right?: React.ReactNode
  /** 0-100 — renders a thin accent line along the toolbar's bottom edge (Cull's cull-progress indicator). */
  progressPct?: number
}

export function StageToolbar({ icon: Icon, stageLabel, sessionName, children, right, progressPct }: StageToolbarProps) {
  const openSettings = useStore(s => s.openSettings)

  return (
    <div
      data-tauri-drag-region
      className="relative flex items-center gap-3 pl-[80px] pr-3 h-[52px] vibrancy-panel border-b border-border flex-shrink-0"
    >
      <Icon className="text-lo text-2xl flex-shrink-0" />
      <span className="text-mid text-xs font-medium flex-shrink-0">{stageLabel}</span>

      {sessionName && (
        <>
          <VDiv />
          <span className="text-mid text-xs truncate max-w-[200px] flex-shrink-0">{sessionName}</span>
        </>
      )}

      {children}

      <div className="flex-1" />

      {right ?? (
        <button
          onClick={openSettings}
          className="w-[36px] h-[36px] flex items-center justify-center rounded-lg text-lo hover:text-mid hover:bg-surface text-2xl transition-colors duration-100"
        >
          <IconSettings />
        </button>
      )}

      {progressPct !== undefined && progressPct > 0 && (
        <div className="absolute bottom-0 left-0 right-0 h-[2px] overflow-hidden">
          <div
            className="h-full transition-all duration-500 ease-out"
            style={{
              width: `${progressPct}%`,
              background: 'rgb(var(--color-accent-rgb))',
              opacity: progressPct >= 100 ? 0 : 1,
            }}
          />
        </div>
      )}
    </div>
  )
}

/** Vertical separator for inline flex rows. */
export function VDiv({ tall }: { tall?: boolean } = {}) {
  return (
    <div
      className="flex-shrink-0 bg-border"
      style={{ width: 1, height: tall ? 22 : 16 }}
    />
  )
}
