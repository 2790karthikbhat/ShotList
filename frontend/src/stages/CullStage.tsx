import { useEffect, useCallback } from 'react'
import { useStore } from '../store'
import { CULL_STATE_COLORS } from '../types'
import { Filmstrip } from '../components/Filmstrip'
import { FilterBar } from '../components/FilterBar'
import { StageToolbar, VDiv } from '../components/StageToolbar'
import { MetadataPill, StarRating } from '../components/MetadataPill'
import {
  IconPhoto,
  IconFolderPlus,
  IconDownload,
  IconZap,
  IconCheck,
  IconX,
  IconCircle,
  IconFlag,
} from '../components/icons'

// ── Keyboard handler ───────────────────────────────────────────────────────────

function useKeyboard() {
  const navigate     = useStore(s => s.navigate)
  const setCullState = useStore(s => s.setCullState)
  const setRating    = useStore(s => s.setRating)
  const currentId    = useStore(s => s.currentId)
  const images       = useStore(s => s.images)
  const session      = useStore(s => s.session)

  const handleKey = useCallback((e: KeyboardEvent) => {
    if (!session || !currentId) return
    if ((e.target as HTMLElement).tagName === 'INPUT') return
    const img = images[currentId]
    if (!img) return

    switch (e.key) {
      case 'ArrowRight': case 'l': case 'L':
        e.preventDefault(); navigate('next'); break
      case 'ArrowLeft': case 'j': case 'J':
        e.preventDefault(); navigate('prev'); break
      case 'p': case 'P':  setCullState(img.path, 'pick'); break
      case 'x': case 'X': case 'Delete': case 'Backspace':
        setCullState(img.path, 'reject'); break
      case 'u': case 'U': setCullState(img.path, 'unreviewed'); break
      case '6': setCullState(img.path, 'flagged'); break
      case '1': setRating(img.path, 1); break
      case '2': setRating(img.path, 2); break
      case '3': setRating(img.path, 3); break
      case '4': setRating(img.path, 4); break
      case '5': setRating(img.path, 5); break
      case '0': setRating(img.path, 0); break
    }
  }, [session, currentId, images, navigate, setCullState, setRating])

  useEffect(() => {
    window.addEventListener('keydown', handleKey)
    return () => window.removeEventListener('keydown', handleKey)
  }, [handleKey])
}

// ── Toolbar ────────────────────────────────────────────────────────────────────

function StatChip({
  value,
  color,
  icon: Icon,
}: {
  value: number
  color: string
  icon: (props: React.SVGProps<SVGSVGElement>) => JSX.Element
}) {
  return (
    <span className="inline-flex items-center gap-0.5 text-[11px] font-mono tabular-nums" style={{ color }}>
      <Icon />
      {value}
    </span>
  )
}

function Toolbar() {
  const session      = useStore(s => s.session)
  const importFolder = useStore(s => s.importFolder)
  const exportPicks  = useStore(s => s.exportPicks)
  const autoCull     = useStore(s => s.autoCull)
  const progressPct  = session?.progressPct ?? 0

  return (
    <StageToolbar icon={IconPhoto} stageLabel="Cull" sessionName={session?.name} progressPct={progressPct}>
      {!session && (
        <button
          onClick={importFolder}
          className="flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg text-xs text-mid
                     hover:text-hi hover:bg-surface transition-colors duration-100"
        >
          <IconFolderPlus className="text-accent" style={{ fontSize: 15 }} />
          <span>Import</span>
        </button>
      )}

      {session && (
        <>
          <FilterBar />
          <VDiv tall />
          {/* Cull stats */}
          <div className="flex items-center gap-2.5">
            <StatChip value={session.picks}      color="#50C378" icon={IconCheck} />
            <StatChip value={session.rejects}    color="#D24646" icon={IconX} />
            <StatChip value={session.unreviewed} color="#555555" icon={IconCircle} />
            {session.flagged > 0 && (
              <StatChip value={session.flagged} color="#DCA032" icon={IconFlag} />
            )}
          </div>
          <VDiv tall />
          {/* Secondary actions */}
          <button
            onClick={exportPicks}
            className="flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg text-xs text-mid
                       hover:text-hi hover:bg-surface transition-colors duration-100"
          >
            <IconDownload className="text-lo text-xl" /> Export picks
          </button>
          <button
            onClick={autoCull}
            className="flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg text-xs text-mid
                       hover:text-hi hover:bg-surface transition-colors duration-100"
          >
            <IconZap className="text-lo text-xl"/> Auto-cull
          </button>
        </>
      )}
    </StageToolbar>
  )
}

// FilterBar (rendered inline above) and Filmstrip are imported from shared components.

// ── Viewer ─────────────────────────────────────────────────────────────────────

function Viewer() {
  const session       = useStore(s => s.session)
  const currentId     = useStore(s => s.currentId)
  const images        = useStore(s => s.images)
  const viewerSrc     = useStore(s => s.viewerSrc)
  const viewerLoading = useStore(s => s.viewerLoading)
  const importFolder  = useStore(s => s.importFolder)

  // ── Empty / onboarding state ───────────────────────────────────────────────
  if (!session || !currentId) {
    return (
      <div className="flex-1 flex flex-col items-center justify-center bg-bg gap-5 select-none">
        {/* App mark */}
        <div
          aria-hidden
          style={{ fontSize: 80, color: 'var(--color-appmark)', fontWeight: 900, lineHeight: 1, letterSpacing: '-4px' }}
        >
          ◈
        </div>

        <div className="text-center space-y-1.5">
          <h1 className="text-xl font-semibold tracking-tight text-hi">ShotList</h1>
          <p className="text-sm text-lo">Your photography culling workflow</p>
        </div>

        <button
          onClick={importFolder}
          className="mt-1 px-7 py-3 rounded-xl text-sm font-semibold
                     bg-accent text-black hover:brightness-110 active:brightness-90
                     transition-all duration-100 shadow-panel"
        >
          Import folder
        </button>

        <p className="text-[11px]" style={{ color: 'var(--color-caption-dim)' }}>
          Supports RAW · JPEG · TIFF
        </p>
      </div>
    )
  }

  const img       = images[currentId]
  if (!img) return null
  const cullColor = CULL_STATE_COLORS[img.cullState]

  return (
    <div className="flex-1 relative overflow-hidden bg-bg p-6">
      {/* Photo card — inset, rounded, shadowed instead of edge-to-edge */}
      <div className="absolute inset-6 rounded-xl overflow-hidden shadow-panel bg-panel/40">
        {viewerSrc ? (
          <img
            src={viewerSrc}
            alt={img.filename}
            className="absolute inset-0 w-full h-full object-contain"
            draggable={false}
          />
        ) : (
          <div className="absolute inset-0 flex items-center justify-center">
            {viewerLoading && (
              <span className="text-lo text-xs tracking-widest animate-pulse">Loading…</span>
            )}
          </div>
        )}

        {/* Cull state badge — top-left */}
        {img.cullState !== 'unreviewed' && (
          <div
            className="absolute top-3 left-3 px-2.5 py-1 rounded-full text-[10px] font-bold
                       uppercase tracking-widest backdrop-blur-vibrancy"
            style={{
              background: 'rgba(0,0,0,0.5)',
              color: cullColor,
              border: `1px solid ${cullColor}55`,
            }}
          >
            {img.cullState}
          </div>
        )}

        {/* HD decode indicator — top-right */}
        {viewerLoading && !viewerSrc && (
          <div className="absolute top-3 right-3 px-2.5 py-1 rounded-full text-[10px] text-lo bg-black/50 backdrop-blur-vibrancy">
            HD loading…
          </div>
        )}

        {/* Rating pill — bottom-left */}
        <div className="absolute bottom-3 left-3 px-3 py-1.5 rounded-full bg-black/50 backdrop-blur-vibrancy">
          <StarRating img={img} />
        </div>
      </div>
    </div>
  )
}


// ── CullStage root ─────────────────────────────────────────────────────────────

export function CullStage() {
  useKeyboard()

  const currentId = useStore(s => s.currentId)
  const images    = useStore(s => s.images)
  const currentImg = currentId ? images[currentId] : undefined

  return (
    <div className="flex flex-col flex-1 overflow-hidden">
      <Toolbar />
      <Viewer />
      <Filmstrip trailing={currentImg && <MetadataPill img={currentImg} />} />
    </div>
  )
}
