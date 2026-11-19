// Shared filmstrip used by Cull/Analyse/Develop/Caption. Displays all visible
// images as thumbnails; clicking one navigates to it. `trailing` renders a
// pinned element (e.g. Cull's metadata pill) at the row's end, outside the
// horizontal scroll — used instead of a separate metadata bar.

import { useEffect, useRef } from 'react'
import { useStore } from '../store'
import { CULL_STATE_COLORS } from '../types'
import * as ipc from '../lib/ipc'

const STRIP_H = 108   // total strip height in px
const CELL_H  = 76    // thumbnail cell height in px

export function Filmstrip({ trailing }: { trailing?: React.ReactNode } = {}) {
  const session    = useStore(s => s.session)
  const visibleIds = useStore(s => s.visibleIds)
  const images     = useStore(s => s.images)
  const currentId  = useStore(s => s.currentId)
  const thumbnails = useStore(s => s.thumbnails)
  const jumpTo     = useStore(s => s.jumpTo)

  const selectedRef = useRef<HTMLDivElement>(null)
  const requestedRef = useRef<Set<string>>(new Set())

  // Keep the selected cell in view.
  useEffect(() => {
    selectedRef.current?.scrollIntoView({
      behavior: 'smooth',
      block: 'nearest',
      inline: 'center',
    })
  }, [currentId])

  // Request thumbnails that haven't been decoded yet. Deliberately only
  // depends on `visibleIds`, not `thumbnails` — depending on `thumbnails`
  // meant every single incoming thumbnail re-triggered this effect, which
  // re-requested the entire still-missing set again even though those
  // requests were already in flight. That flooded the backend's bounded
  // decode channel with redundant duplicates until it saturated and started
  // silently dropping unrelated requests (e.g. the viewer's high-res decode
  // for whatever image you'd navigated to, leaving it stuck on "Loading…").
  // `requestedRef` tracks what's already been dispatched so we don't re-send.
  useEffect(() => {
    const missing = visibleIds.filter(id => !thumbnails[id] && !requestedRef.current.has(id))
    if (missing.length > 0) {
      missing.forEach(id => requestedRef.current.add(id))
      ipc.requestThumbnails(missing).catch(() => {})
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [visibleIds])

  if (!session) {
    return (
      <div
        className="flex items-center justify-center vibrancy-panel border-t border-border flex-shrink-0"
        style={{ height: STRIP_H }}
      >
        <span className="text-lo text-[11px]">No session</span>
      </div>
    )
  }

  return (
    <div
      className="flex items-center gap-4 px-3 vibrancy-panel border-t border-border flex-shrink-0"
      style={{ height: STRIP_H }}
    >
      <div className="flex-1 min-w-0 overflow-x-auto overflow-y-hidden h-full">
        <div className="flex gap-2.5 h-full items-center py-[16px]">
          {visibleIds.map(id => {
            const img     = images[id]
            if (!img) return null
            const isCurrent = id === currentId
            const thumb     = thumbnails[id]
            const cellW     = Math.round(CELL_H * 1.5)
            const cullColor = img.cullState !== 'unreviewed'
              ? CULL_STATE_COLORS[img.cullState]
              : null

            return (
              <div
                key={id}
                ref={isCurrent ? selectedRef : null}
                onClick={() => jumpTo(img.fullImageIndex)}
                className={`group relative flex-shrink-0 cursor-pointer rounded-xl overflow-hidden transition-transform duration-150 ${
                  isCurrent ? '-translate-y-1.5 scale-[1.03]' : 'hover:-translate-y-1 hover:scale-[1.02]'
                }`}
                style={{
                  width:  cellW,
                  height: CELL_H,
                  background: 'var(--color-filmstrip-placeholder)',
                  outline: isCurrent ? '2px solid rgb(var(--color-accent-rgb))' : undefined,
                  outlineOffset: '-2px',
                  boxShadow: isCurrent ? '0 14px 26px -12px rgb(var(--color-accent-rgb) / 0.45)' : undefined,
                }}
              >
                {/* Thumbnail or shimmer skeleton */}
                {thumb ? (
                  <img
                    src={thumb}
                    alt={img.filename}
                    className="w-full h-full object-cover"
                    draggable={false}
                  />
                ) : (
                  <div className="w-full h-full shimmer" />
                )}

                {/* Hover darkening */}
                {!isCurrent && (
                  <div className="absolute inset-0 bg-black/0 group-hover:bg-black/25 transition-colors duration-100 rounded-xl" />
                )}

                {/* Cull state: 3 px colored top border (Lightroom-style flag) */}
                {cullColor && (
                  <div
                    className="absolute top-0 left-0 right-0 h-[3px] rounded-t-xl"
                    style={{ background: cullColor }}
                  />
                )}

                {/* Face count chip — top-right */}
                {img.faceCount > 0 && (
                  <div
                    className="absolute top-1 right-1 min-w-[15px] h-[15px] rounded-full
                               flex items-center justify-center text-[9px] font-bold text-black px-1"
                    style={{ background: 'rgba(74,200,255,0.9)' }}
                  >
                    {img.faceCount}
                  </div>
                )}

                {/* Burst / group chip — bottom-right, above focus bar */}
                {img.groupSize && img.groupSize > 1 && (
                  <div
                    className="absolute right-1 min-w-[15px] h-[15px] rounded-full
                               flex items-center justify-center text-[9px] font-bold text-black px-1"
                    style={{ bottom: 6, background: 'rgba(180,120,32,0.9)' }}
                  >
                    {img.groupSize}
                  </div>
                )}

                {/* Focus meter bar */}
                {img.focusScore !== null && (
                  <div
                    className="absolute bottom-0 left-0 right-0 h-[3px]"
                    style={{ background: 'rgba(0,0,0,0.5)' }}
                  >
                    <div
                      className="h-full"
                      style={{
                        width: `${img.focusScore * 100}%`,
                        background:
                          img.focusScore >= 0.40 ? '#50C378'
                          : img.focusScore >= 0.18 ? '#DCA032'
                          : '#D24646',
                      }}
                    />
                  </div>
                )}
              </div>
            )
          })}
        </div>
      </div>

      {trailing && <div className="flex-shrink-0">{trailing}</div>}
    </div>
  )
}
