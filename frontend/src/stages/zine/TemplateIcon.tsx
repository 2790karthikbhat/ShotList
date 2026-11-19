// Small 48×34 px diagrams showing each zine template's layout visually.

import React from 'react'
import type { TemplateId } from '../../types'

export function TemplateIcon({ id, size = 48 }: { id: TemplateId; size?: number }) {
  const h = Math.round(size * 0.7)
  const s = { stroke: 'currentColor', strokeWidth: 1, fill: 'none' } as const
  const img = { fill: '#888' } as const
  const txt = { fill: '#bbb' } as const

  const icons: Record<TemplateId, React.ReactNode> = {
    fullBleed: (
      <svg width={size} height={h} viewBox={`0 0 ${size} ${h}`}>
        <rect x={0} y={0} width={size} height={h} {...img} opacity={0.5} />
        <rect x={0} y={h * 0.78} width={size} height={h * 0.22} {...txt} opacity={0.7} />
      </svg>
    ),
    top75: (
      <svg width={size} height={h} viewBox={`0 0 ${size} ${h}`}>
        <rect x={0} y={0} width={size} height={h * 0.75} {...img} opacity={0.5} />
        <rect x={0} y={h * 0.75} width={size} height={h * 0.25} {...txt} opacity={0.3} />
        <line x1={4} y1={h * 0.79} x2={size - 4} y2={h * 0.79} {...s} strokeOpacity={0.4} />
        <line x1={4} y1={h * 0.87} x2={size * 0.6} y2={h * 0.87} {...s} strokeOpacity={0.3} />
      </svg>
    ),
    left50: (
      <svg width={size} height={h} viewBox={`0 0 ${size} ${h}`}>
        <rect x={0} y={0} width={size * 0.5} height={h} {...img} opacity={0.5} />
        <rect x={size * 0.5} y={0} width={size * 0.5} height={h} {...txt} opacity={0.2} />
        <line x1={size * 0.54} y1={h * 0.2} x2={size * 0.94} y2={h * 0.2} {...s} strokeOpacity={0.5} />
        <line x1={size * 0.54} y1={h * 0.32} x2={size * 0.9} y2={h * 0.32} {...s} strokeOpacity={0.35} />
        <line x1={size * 0.54} y1={h * 0.44} x2={size * 0.8} y2={h * 0.44} {...s} strokeOpacity={0.25} />
      </svg>
    ),
    right50: (
      <svg width={size} height={h} viewBox={`0 0 ${size} ${h}`}>
        <rect x={0} y={0} width={size * 0.5} height={h} {...txt} opacity={0.2} />
        <rect x={size * 0.5} y={0} width={size * 0.5} height={h} {...img} opacity={0.5} />
        <line x1={size * 0.06} y1={h * 0.2} x2={size * 0.46} y2={h * 0.2} {...s} strokeOpacity={0.5} />
        <line x1={size * 0.06} y1={h * 0.32} x2={size * 0.42} y2={h * 0.32} {...s} strokeOpacity={0.35} />
        <line x1={size * 0.06} y1={h * 0.44} x2={size * 0.36} y2={h * 0.44} {...s} strokeOpacity={0.25} />
      </svg>
    ),
    gridTwo: (
      <svg width={size} height={h} viewBox={`0 0 ${size} ${h}`}>
        <rect x={0} y={0} width={size * 0.49} height={h} {...img} opacity={0.5} />
        <rect x={size * 0.51} y={0} width={size * 0.49} height={h} {...img} opacity={0.35} />
      </svg>
    ),
    titlePage: (
      <svg width={size} height={h} viewBox={`0 0 ${size} ${h}`}>
        <rect x={0} y={0} width={size} height={h} fill="#2a2a2a" />
        <line x1={size * 0.15} y1={h * 0.38} x2={size * 0.85} y2={h * 0.38} stroke="#ccc" strokeWidth={1.5} strokeOpacity={0.7} />
        <line x1={size * 0.15} y1={h * 0.52} x2={size * 0.75} y2={h * 0.52} stroke="#aaa" strokeWidth={1} strokeOpacity={0.4} />
        <line x1={size * 0.15} y1={h * 0.62} x2={size * 0.65} y2={h * 0.62} stroke="#aaa" strokeWidth={1} strokeOpacity={0.3} />
      </svg>
    ),
    pullQuote: (
      <svg width={size} height={h} viewBox={`0 0 ${size} ${h}`}>
        <rect x={0} y={0} width={size} height={h} {...img} opacity={0.55} />
        <rect x={0} y={h * 0.62} width={size} height={h * 0.38} fill="#111" opacity={0.75} />
        <line x1={4} y1={h * 0.72} x2={size - 4} y2={h * 0.72} stroke="#fff" strokeWidth={1.4} strokeOpacity={0.8} />
        <line x1={4} y1={h * 0.84} x2={size * 0.65} y2={h * 0.84} stroke="#fff" strokeWidth={0.8} strokeOpacity={0.45} />
      </svg>
    ),
    imageLead: (
      <svg width={size} height={h} viewBox={`0 0 ${size} ${h}`}>
        <rect x={0} y={0} width={size} height={h * 0.6} {...img} opacity={0.55} />
        <rect x={0} y={h * 0.6} width={size} height={h * 0.4} fill="#f5f5f5" />
        <line x1={4} y1={h * 0.68} x2={size - 4} y2={h * 0.68} stroke="#111" strokeWidth={1.8} strokeOpacity={0.7} />
        <line x1={4} y1={h * 0.78} x2={size * 0.75} y2={h * 0.78} stroke="#555" strokeWidth={0.8} strokeOpacity={0.5} />
        <line x1={4} y1={h * 0.86} x2={size * 0.6} y2={h * 0.86} stroke="#555" strokeWidth={0.8} strokeOpacity={0.35} />
      </svg>
    ),
    panoramic: (
      <svg width={size} height={h} viewBox={`0 0 ${size} ${h}`}>
        <rect x={0} y={0} width={size} height={h} fill="#f5f5f5" />
        <rect x={0} y={h * 0.18} width={size} height={h * 0.52} {...img} opacity={0.5} />
        <line x1={4} y1={h * 0.78} x2={size - 4} y2={h * 0.78} {...s} strokeOpacity={0.5} />
        <line x1={4} y1={h * 0.88} x2={size * 0.6} y2={h * 0.88} {...s} strokeOpacity={0.35} />
      </svg>
    ),
    editorial: (
      <svg width={size} height={h} viewBox={`0 0 ${size} ${h}`}>
        <rect x={0} y={0} width={size * 0.65} height={h} {...img} opacity={0.55} />
        <rect x={size * 0.65} y={0} width={size * 0.35} height={h} fill="#f5f5f5" />
        <line x1={size * 0.68} y1={h * 0.28} x2={size - 3} y2={h * 0.28} stroke="#111" strokeWidth={1.5} strokeOpacity={0.7} />
        <line x1={size * 0.68} y1={h * 0.38} x2={size - 3} y2={h * 0.38} {...s} strokeOpacity={0.5} />
        <line x1={size * 0.68} y1={h * 0.46} x2={size - 3} y2={h * 0.46} {...s} strokeOpacity={0.4} />
        <line x1={size * 0.68} y1={h * 0.54} x2={size - 5} y2={h * 0.54} {...s} strokeOpacity={0.3} />
        <line x1={size * 0.68} y1={h * 0.62} x2={size - 4} y2={h * 0.62} {...s} strokeOpacity={0.25} />
      </svg>
    ),
    triptych: (
      <svg width={size} height={h} viewBox={`0 0 ${size} ${h}`}>
        {[0, 1, 2].map(i => (
          <rect key={i} x={i * size / 3} y={0} width={size / 3 - 1} height={h} {...img} opacity={0.5 - i * 0.05} />
        ))}
        <line x1={size / 3 - 0.5} y1={0} x2={size / 3 - 0.5} y2={h} stroke="#fff" strokeWidth={1} />
        <line x1={size * 2 / 3 - 0.5} y1={0} x2={size * 2 / 3 - 0.5} y2={h} stroke="#fff" strokeWidth={1} />
      </svg>
    ),
    grid4x: (
      <svg width={size} height={h} viewBox={`0 0 ${size} ${h}`}>
        <rect x={0}          y={h / 2}  width={size / 2 - 1} height={h / 2} {...img} opacity={0.55} />
        <rect x={size / 2}  y={h / 2}  width={size / 2}     height={h / 2} {...img} opacity={0.45} />
        <rect x={0}          y={0}       width={size / 2 - 1} height={h / 2 - 1} {...img} opacity={0.40} />
        <rect x={size / 2}  y={0}       width={size / 2}     height={h / 2 - 1} {...img} opacity={0.50} />
        <line x1={size / 2} y1={0} x2={size / 2} y2={h} stroke="#fff" strokeWidth={1} />
        <line x1={0} y1={h / 2} x2={size} y2={h / 2} stroke="#fff" strokeWidth={1} />
      </svg>
    ),
    featureSpread: (
      <svg width={size} height={h} viewBox={`0 0 ${size} ${h}`}>
        <rect x={0}         y={h * 0.3} width={size}     height={h * 0.7} {...img} opacity={0.55} />
        <rect x={0}         y={0}       width={size / 2 - 1} height={h * 0.3 - 1} {...img} opacity={0.45} />
        <rect x={size / 2} y={0}       width={size / 2}  height={h * 0.3 - 1} {...img} opacity={0.40} />
        <line x1={0} y1={h * 0.3} x2={size} y2={h * 0.3} stroke="#fff" strokeWidth={1} />
        <line x1={size / 2} y1={0} x2={size / 2} y2={h * 0.3} stroke="#fff" strokeWidth={1} />
      </svg>
    ),
    photoEssay: (
      <svg width={size} height={h} viewBox={`0 0 ${size} ${h}`}>
        <rect x={0}            y={0}      width={size / 2 - 1} height={h}      {...img} opacity={0.55} />
        <rect x={size / 2}     y={0}      width={size / 4 - 1} height={h / 2 - 1} {...img} opacity={0.45} />
        <rect x={size * 3 / 4} y={0}      width={size / 4}     height={h / 2 - 1} {...img} opacity={0.40} />
        <rect x={size / 2}     y={h / 2}  width={size / 4 - 1} height={h / 2}     {...img} opacity={0.38} />
        <rect x={size * 3 / 4} y={h / 2}  width={size / 4}     height={h / 2}     {...img} opacity={0.43} />
        <line x1={size / 2} y1={0} x2={size / 2} y2={h} stroke="#fff" strokeWidth={1} />
        <line x1={size / 2} y1={h / 2} x2={size} y2={h / 2} stroke="#fff" strokeWidth={1} />
        <line x1={size * 3 / 4} y1={0} x2={size * 3 / 4} y2={h} stroke="#fff" strokeWidth={0.5} />
      </svg>
    ),
    contactSheet: (
      <svg width={size} height={h} viewBox={`0 0 ${size} ${h}`}>
        <rect x={0} y={0} width={size} height={h} fill="#e8e8e8" />
        {Array.from({ length: 9 }, (_, i) => {
          const col = i % 3
          const row = Math.floor(i / 3)
          const cw = (size - 4) / 3
          const ch = (h - 4) / 3
          return <rect key={i} x={1 + col * (cw + 1)} y={1 + row * (ch + 1)} width={cw} height={ch} {...img} opacity={0.4 + (i % 3) * 0.06} />
        })}
      </svg>
    ),
  }
  return <>{icons[id]}</>
}
