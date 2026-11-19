// Filmstrip metadata pill — identity/EXIF/technical info (+ optional
// keyboard hints) shown as a card pinned to the end of the filmstrip.
// Originally Cull-only (replacing the old standalone metadata bar); shared
// with Develop via the `showHints` flag so Develop can show the same
// filename/EXIF/rating details without the cull-decision (P/X/U) hints,
// which don't apply there.

import { useStore } from '../store'
import type { ImageInfo } from '../types'
import { IconTriangle, IconTarget, IconUser, IconStar } from './icons'
import { VDiv } from './StageToolbar'

export function MetadataPill({ img, showHints = true }: { img: ImageInfo; showHints?: boolean }) {
  const exifParts: string[] = []
  if (img.iso)           exifParts.push(`ISO ${img.iso}`)
  if (img.shutterSpeed)  exifParts.push(img.shutterSpeed)
  if (img.aperture)      exifParts.push(`f/${img.aperture.toFixed(1)}`)
  if (img.focalLengthMm) exifParts.push(`${Math.round(img.focalLengthMm)} mm`)

  const fs         = img.focusScore
  const focusColor =
    fs === null  ? 'rgb(var(--color-lo-rgb))'
    : fs >= 0.40 ? '#50C378'
    : fs >= 0.18 ? '#DCA032'
    : '#D24646'

  return (
    <div className="bg-surface rounded-xl px-4 py-2.5 min-w-[300px] max-w-[400px] text-[11px] select-none">
      {/* Identity */}
      <div className="flex items-center gap-2 mb-1.5">
        <span className="text-lo font-mono tabular-nums">
          {String(img.fullImageIndex + 1).padStart(3, '0')}
        </span>
        <VDiv />
        <span className="text-hi font-mono font-medium truncate">{img.filename}</span>
        <span className="text-lo flex-shrink-0 ml-auto">{img.sizeLabel}</span>
      </div>

      {/* EXIF + technical */}
      <div className="flex items-center gap-2.5 flex-wrap">
        {exifParts.length > 0
          ? exifParts.map((part, i) => (
              <span key={i} className="text-mid tabular-nums flex-shrink-0">{part}</span>
            ))
          : <span className="text-lo">No EXIF data</span>
        }

        {img.exposureBlown && (
          <span className="flex items-center gap-0.5 flex-shrink-0" style={{ color: '#FFAA44' }}>
            <IconTriangle /> blown
          </span>
        )}
        {img.exposureCrushed && (
          <span className="flex items-center gap-0.5 flex-shrink-0" style={{ color: '#AA70FF' }}>
            <IconTriangle style={{ transform: 'rotate(180deg)' }} /> crushed
          </span>
        )}

        <VDiv />
        <span style={{ color: focusColor }} className="flex items-center gap-0.5 tabular-nums flex-shrink-0">
          <IconTarget />{fs !== null ? `${Math.round(fs * 100)}%` : '—'}
        </span>

        {img.faceCount > 0 && (
          <span style={{ color: '#4AC8FF' }} className="flex items-center gap-0.5 flex-shrink-0">
            <IconUser />{img.faceCount}
          </span>
        )}
      </div>

      {/* Keyboard hints — Cull only (pick/reject/unreviewed + rating shortcuts
          don't apply outside Cull, so Develop opts out via showHints={false}). */}
      {showHints && (
        <div className="hidden xl:flex items-center gap-1.5 mt-1.5 pt-1.5 border-t border-border">
          <Kbd>P</Kbd>
          <Kbd>X</Kbd>
          <Kbd>U</Kbd>
          <span className="text-lo/50 text-[10px]">cull</span>
          <Kbd>1–5</Kbd>
          <span className="text-lo/50 text-[10px]">stars</span>
          <Kbd>←→</Kbd>
          <span className="text-lo/50 text-[10px]">nav</span>
        </div>
      )}
    </div>
  )
}

function Kbd({ children }: { children: React.ReactNode }) {
  return (
    <span
      className="px-1 py-px rounded text-[10px] font-mono text-lo leading-none"
      style={{ background: 'var(--color-kbd-bg)', border: '1px solid rgb(var(--color-border-rgb))' }}
    >
      {children}
    </span>
  )
}

export function StarRating({ img }: { img: ImageInfo }) {
  const setRating = useStore(s => s.setRating)
  return (
    <div className="flex items-center gap-0.5">
      {[1, 2, 3, 4, 5].map(star => (
        <button
          key={star}
          onClick={e => { e.stopPropagation(); setRating(img.path, star === img.rating ? 0 : star) }}
          className="text-sm leading-none transition-transform duration-75 hover:scale-125"
          style={{ color: star <= img.rating ? '#DCA032' : 'var(--color-star-empty)' }}
          title={`${star} star${star !== 1 ? 's' : ''}`}
        >
          <IconStar fill={star <= img.rating ? 'currentColor' : 'none'} />
        </button>
      ))}
    </div>
  )
}
