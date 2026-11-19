// Phase 1 — Technical Analyser
// Displays the current image alongside a right-hand panel showing:
//   • RGB + luminance histogram (canvas)
//   • Ansel Adams Zone System distribution (bar chart)
//   • Spatial focus heatmap (8 × 5 grid of Laplacian energies)
//   • Scalar stats: blown %, crushed %, mean luma, colour temperature

import { useEffect, useRef, useCallback, useState } from 'react'
import { useStore } from '../store'
import { type ImageAnalysis } from '../types'
import { Filmstrip } from '../components/Filmstrip'
import { MetadataPill } from '../components/MetadataPill'
import { FilterBar } from '../components/FilterBar'
import { StageToolbar } from '../components/StageToolbar'
import { IconChartBar, IconChevronLeft, IconChevronRight } from '../components/icons'
import * as ipc from '../lib/ipc'

// ── Keyboard: navigation only (no cull decisions here) ────────────────────────

function useNavigationKeys() {
  const navigate = useStore(s => s.navigate)
  const session  = useStore(s => s.session)

  const handleKey = useCallback((e: KeyboardEvent) => {
    if (!session) return
    if ((e.target as HTMLElement).tagName === 'INPUT') return
    switch (e.key) {
      case 'ArrowRight': case 'l': case 'L': e.preventDefault(); navigate('next'); break
      case 'ArrowLeft':  case 'j': case 'J': e.preventDefault(); navigate('prev'); break
    }
  }, [session, navigate])

  useEffect(() => {
    window.addEventListener('keydown', handleKey)
    return () => window.removeEventListener('keydown', handleKey)
  }, [handleKey])
}

// ── Toolbar ───────────────────────────────────────────────────────────────────

function Toolbar() {
  const session  = useStore(s => s.session)
  const navigate = useStore(s => s.navigate)

  return (
    <StageToolbar icon={IconChartBar} stageLabel="Analyse" sessionName={session?.name}>
      {session && (
        <>
          <FilterBar />
          {/* Prev / next navigation arrows */}
          <button
            onClick={() => navigate('prev')}
            className="w-[32px] h-[32px] flex items-center justify-center rounded-lg text-mid hover:text-hi hover:bg-surface transition-colors text-xl"
            title="Previous (← / J)"
          >
            <IconChevronLeft />
          </button>
          <button
            onClick={() => navigate('next')}
            className="w-[32px] h-[32px] flex items-center justify-center rounded-lg text-mid hover:text-hi hover:bg-surface transition-colors text-xl"
            title="Next (→ / L)"
          >
            <IconChevronRight />
          </button>
        </>
      )}
    </StageToolbar>
  )
}

// ── Viewer (left panel) ───────────────────────────────────────────────────────

function Viewer() {
  const session       = useStore(s => s.session)
  const currentId     = useStore(s => s.currentId)
  const images        = useStore(s => s.images)
  const viewerSrc     = useStore(s => s.viewerSrc)
  const viewerLoading = useStore(s => s.viewerLoading)

  if (!session || !currentId) {
    return (
      <div className="flex-1 flex items-center justify-center bg-bg text-lo text-sm">
        Import a folder in the Cull stage first.
      </div>
    )
  }

  const img = images[currentId]
  if (!img) return null

  return (
    <div className="flex-1 relative bg-bg overflow-hidden p-6">
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
      </div>
    </div>
  )
}

// ── Analysis panel (right panel) ──────────────────────────────────────────────

function AnalysisPanel() {
  const currentId = useStore(s => s.currentId)
  const [analysis, setAnalysis] = useState<ImageAnalysis | null>(null)
  const [loading,  setLoading]  = useState(false)
  const [error,    setError]    = useState<string | null>(null)

  useEffect(() => {
    if (!currentId) { setAnalysis(null); setError(null); return }
    setLoading(true)
    setError(null)
    ipc.getImageAnalysis(currentId)
      .then(a => { setAnalysis(a); setLoading(false) })
      .catch(e => { setError(String(e)); setLoading(false) })
  }, [currentId])

  if (!currentId) {
    return (
      <div className="flex items-center justify-center h-full text-lo text-xs px-4 text-center">
        No image selected
      </div>
    )
  }

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full">
        <span className="text-lo text-xs animate-pulse">Analysing…</span>
      </div>
    )
  }

  if (error) {
    return (
      <div className="flex flex-col items-center justify-center h-full gap-2 px-4 text-center">
        <span className="text-lo text-xs">{error}</span>
      </div>
    )
  }

  if (!analysis) return null

  return (
    <div className="flex flex-col gap-3 overflow-y-auto h-full p-3">
      <HistogramSection  analysis={analysis} />
      <ZoneSection       analysis={analysis} />
      <FocusMapSection   analysis={analysis} />
      <StatsSection      analysis={analysis} />
    </div>
  )
}

// ── Histogram ─────────────────────────────────────────────────────────────────

function HistogramSection({ analysis }: { analysis: ImageAnalysis }) {
  const canvasRef = useRef<HTMLCanvasElement>(null)

  useEffect(() => {
    const canvas = canvasRef.current
    if (!canvas) return
    const ctx = canvas.getContext('2d')
    if (!ctx) return

    const W = canvas.width
    const H = canvas.height

    // Canvas fills can't use CSS variables directly — read the current
    // theme's values at draw-time instead.
    const rootStyle = getComputedStyle(document.documentElement)
    const canvasBg = rootStyle.getPropertyValue('--color-canvas-bg').trim() || '#0d0d0d'
    const lumaStroke = rootStyle.getPropertyValue('--color-canvas-luma-stroke').trim() || 'rgba(255,255,255,0.55)'

    // Background
    ctx.fillStyle = canvasBg
    ctx.fillRect(0, 0, W, H)

    // Draw a channel as a filled area chart
    const drawArea = (data: number[], color: string) => {
      ctx.beginPath()
      ctx.moveTo(0, H)
      for (let i = 0; i < 256; i++) {
        const x = (i / 255) * W
        const y = H - data[i] * H * 0.96
        ctx.lineTo(x, y)
      }
      ctx.lineTo(W, H)
      ctx.closePath()
      ctx.fillStyle = color
      ctx.fill()
    }

    // Draw B under G under R; luminance as white line on top
    drawArea(analysis.histB, 'rgba(50,90,255,0.55)')
    drawArea(analysis.histG, 'rgba(50,200,60,0.55)')
    drawArea(analysis.histR, 'rgba(255,55,55,0.55)')

    // Luminance as white stroke line
    ctx.beginPath()
    for (let i = 0; i < 256; i++) {
      const x = (i / 255) * W
      const y = H - analysis.histLum[i] * H * 0.96
      if (i === 0) ctx.moveTo(x, y); else ctx.lineTo(x, y)
    }
    ctx.strokeStyle = lumaStroke
    ctx.lineWidth = 1
    ctx.stroke()

    // Blown / crushed indicator lines
    ctx.strokeStyle = 'rgba(255,150,50,0.4)'
    ctx.lineWidth = 1
    ctx.beginPath(); ctx.moveTo(W - 1, 0); ctx.lineTo(W - 1, H); ctx.stroke()
    ctx.strokeStyle = 'rgba(150,100,255,0.4)'
    ctx.beginPath(); ctx.moveTo(1, 0);     ctx.lineTo(1, H);     ctx.stroke()
  }, [analysis])

  return (
    <section className="bg-surface rounded-xl p-3.5">
      <SectionLabel>Histogram</SectionLabel>
      <canvas
        ref={canvasRef}
        width={256}
        height={90}
        className="w-full rounded mt-2"
        style={{ display: 'block', imageRendering: 'pixelated' }}
      />
      {/* Channel legend */}
      <div className="flex items-center gap-3 mt-1.5">
        {[
          { color: 'rgba(255,55,55,0.8)',  label: 'R' },
          { color: 'rgba(50,200,60,0.8)',  label: 'G' },
          { color: 'rgba(50,90,255,0.8)',  label: 'B' },
          { color: 'rgba(200,200,200,0.7)', label: 'Lum' },
        ].map(c => (
          <span key={c.label} className="flex items-center gap-1 text-[10px] text-lo">
            <span className="w-2 h-2 rounded-sm" style={{ background: c.color }} />
            {c.label}
          </span>
        ))}
      </div>
    </section>
  )
}

// ── Zone System ───────────────────────────────────────────────────────────────

const ZONE_LABELS = ['0', 'I', 'II', 'III', 'IV', 'V', 'VI', 'VII', 'VIII', 'IX', 'X']
// Roman numeral label abbreviations for narrow display
const ZONE_SHORT  = ['0', 'I', 'II', 'III', 'IV', 'V', 'VI', 'VII', 'VIII', 'IX', 'X']

function ZoneSection({ analysis }: { analysis: ImageAnalysis }) {
  const zones = analysis.zonePcts
  const maxPct = Math.max(...zones)

  return (
    <section className="bg-surface rounded-xl p-3.5">
      <SectionLabel>Zone System (Ansel Adams)</SectionLabel>
      <div className="mt-2 flex items-end gap-px" style={{ height: 72 }}>
        {zones.map((pct, i) => {
          // Zone 0 = black (0), Zone X = white (255); linear
          const grayVal = Math.round((i / 10) * 255)
          const barH = maxPct > 0 ? (pct / maxPct) * 100 : 0

          return (
            <div
              key={i}
              className="flex-1 flex flex-col justify-end items-center relative"
              style={{ height: '100%' }}
              title={`Zone ${ZONE_LABELS[i]}: ${(pct * 100).toFixed(1)}%`}
            >
              {/* Bar */}
              <div
                className="w-full rounded-t-sm"
                style={{
                  height: `${Math.max(barH, pct > 0 ? 2 : 0)}%`,
                  background: `rgb(${grayVal},${grayVal},${grayVal})`,
                  border: grayVal === 0 ? '1px solid rgb(var(--color-border-rgb))' : undefined,
                  boxSizing: 'border-box',
                  minHeight: pct > 0 ? 2 : 0,
                }}
              />
            </div>
          )
        })}
      </div>

      {/* Zone labels */}
      <div className="flex gap-px mt-1">
        {ZONE_SHORT.map((lbl, i) => (
          <div key={i} className="flex-1 text-center text-[9px] text-lo leading-none">
            {lbl}
          </div>
        ))}
      </div>

      {/* Key zone callout: Zone V (middle gray) percentage */}
      <p className="text-[10px] text-lo mt-1.5">
        Zone V (middle gray):&thinsp;
        <span className="text-mid">{(zones[5] * 100).toFixed(1)}%</span>
        &emsp;
        Highlights (VIII–X):&thinsp;
        <span style={{ color: '#FFAA44' }}>
          {((zones[8] + zones[9] + zones[10]) * 100).toFixed(1)}%
        </span>
        &emsp;
        Shadows (0–II):&thinsp;
        <span style={{ color: '#AA70FF' }}>
          {((zones[0] + zones[1] + zones[2]) * 100).toFixed(1)}%
        </span>
      </p>
    </section>
  )
}

// ── Focus heatmap ─────────────────────────────────────────────────────────────

function FocusMapSection({ analysis }: { analysis: ImageAnalysis }) {
  const { focusMap, focusMapCols: cols, focusMapRows: rows } = analysis

  return (
    <section className="bg-surface rounded-xl p-3.5">
      <SectionLabel>Focus heatmap</SectionLabel>
      <p className="text-[10px] text-lo mt-1 mb-2">
        Red = soft &nbsp;·&nbsp; Yellow = moderate &nbsp;·&nbsp; Green = sharp
      </p>

      {/* Grid sized to match the image aspect ratio */}
      <div
        style={{
          display: 'grid',
          gridTemplateColumns: `repeat(${cols}, 1fr)`,
          gridTemplateRows:    `repeat(${rows}, 1fr)`,
          gap: 2,
          aspectRatio: `${cols} / ${rows}`,
          width: '100%',
        }}
      >
        {focusMap.map((score, idx) => {
          // Red → yellow → green color ramp
          const r = score < 0.5 ? 220 : Math.round(220 * (1 - (score - 0.5) * 2))
          const g = score > 0.5 ? 200 : Math.round(200 * score * 2)
          const pct = Math.round(score * 100)

          return (
            <div
              key={idx}
              className="rounded-sm"
              style={{ background: `rgb(${r},${g},0)` }}
              title={`Cell ${idx + 1}: ${pct}%`}
            />
          )
        })}
      </div>
    </section>
  )
}

// ── Scalar stats ──────────────────────────────────────────────────────────────

function StatsSection({ analysis }: { analysis: ImageAnalysis }) {
  const { blownPct, crushedPct, meanLuma, colorTempK } = analysis

  const tempLabel = colorTempK
    ? colorTempK < 3500 ? `${colorTempK} K — warm (tungsten)`
    : colorTempK < 5000 ? `${colorTempK} K — warm (golden)`
    : colorTempK < 6500 ? `${colorTempK} K — neutral (daylight)`
    : colorTempK < 8000 ? `${colorTempK} K — cool (overcast)`
    : `${colorTempK} K — cool (shade/sky)`
    : 'Unable to estimate (high saturation)'

  const tempColor = !colorTempK ? 'rgb(var(--color-lo-rgb))'
    : colorTempK < 4500 ? '#FFAA44'
    : colorTempK > 7000 ? '#88AAFF'
    : '#90C090'

  return (
    <section className="bg-surface rounded-xl p-3.5">
      <SectionLabel>Exposure stats</SectionLabel>

      <div className="mt-3 space-y-3">
        <StatRow
          label="Mean luma"
          value={`${Math.round(meanLuma * 100)}%`}
          bar={meanLuma}
          color={
            meanLuma < 0.20 ? '#AA70FF'
            : meanLuma > 0.80 ? '#FFAA44'
            : '#50C378'
          }
        />
        <StatRow
          label="Blown highlights"
          value={`${(blownPct * 100).toFixed(1)}%`}
          bar={Math.min(blownPct * 10, 1)}   // saturate at 10%
          color={blownPct > 0.02 ? '#FFAA44' : 'rgb(var(--color-lo-rgb))'}
        />
        <StatRow
          label="Crushed shadows"
          value={`${(crushedPct * 100).toFixed(1)}%`}
          bar={Math.min(crushedPct * 10, 1)}
          color={crushedPct > 0.05 ? '#AA70FF' : 'rgb(var(--color-lo-rgb))'}
        />
      </div>

      {/* Colour temperature */}
      <div className="mt-4">
        <p className="text-[10px] text-lo uppercase tracking-widest mb-1">Colour temperature (est.)</p>
        <p className="text-[11px]" style={{ color: tempColor }}>{tempLabel}</p>
        {colorTempK && (
          <div className="mt-1.5 h-2 rounded-full overflow-hidden" style={{
            background: 'linear-gradient(to right, #FF8800, #FFEE88, #FFFFFF, #88AAFF, #4466FF)',
          }}>
            {/* Marker at the estimated temperature position */}
            <div
              className="h-full w-0.5 bg-white/80 rounded-full"
              style={{
                marginLeft: `${Math.min(Math.max((colorTempK - 1800) / (12000 - 1800), 0), 1) * 100}%`,
              }}
            />
          </div>
        )}
      </div>
    </section>
  )
}

function StatRow({ label, value, bar, color }: {
  label: string
  value: string
  bar: number
  color: string
}) {
  return (
    <div>
      <div className="flex items-center justify-between mb-1">
        <span className="text-[10px] text-lo">{label}</span>
        <span className="text-[11px] font-mono tabular-nums" style={{ color }}>{value}</span>
      </div>
      <div className="h-1 rounded-full bg-surface overflow-hidden">
        <div
          className="h-full rounded-full transition-all duration-500"
          style={{ width: `${bar * 100}%`, background: color }}
        />
      </div>
    </div>
  )
}

// ── Shared primitives ─────────────────────────────────────────────────────────

function SectionLabel({ children }: { children: React.ReactNode }) {
  return (
    <p className="text-[10px] uppercase tracking-widest text-lo">{children}</p>
  )
}

// ── AnalyseStage root ─────────────────────────────────────────────────────────

export function AnalyseStage() {
  useNavigationKeys()

  const currentId = useStore(s => s.currentId)
  const images    = useStore(s => s.images)
  const currentImg = currentId ? images[currentId] : undefined

  return (
    <div className="flex flex-col flex-1 overflow-hidden">
      <Toolbar />

      {/* Middle: viewer (left) + analysis panel (right) */}
      <div className="flex flex-1 overflow-hidden">
        <Viewer />

        {/* Right analysis panel — fixed width, scrollable */}
        <div className="w-[320px] flex-shrink-0 border-l border-border vibrancy-panel overflow-hidden flex flex-col">
          <AnalysisPanel />
        </div>
      </div>

      <Filmstrip trailing={currentImg && <MetadataPill img={currentImg} showHints={false} />} />
    </div>
  )
}
