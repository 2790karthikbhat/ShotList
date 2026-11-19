import React from 'react'

import type { Caption, PaperSize, ZinePage } from '../../types'
import { IconPhoto as ImageIcon } from '../../components/icons'

export function PagePreview({ page, paper, caption, thumbs }: {
  page: ZinePage; paper: PaperSize; caption: Caption | undefined; thumbs: Record<string, string>
}) {
  const t = (id: string | null | undefined) => (id ? thumbs[id] : undefined)
  const thumb = t(page.imageId)
  const dims: Record<PaperSize, [number, number]> = {
    'a4': [210, 297], 'a5': [148, 210], 'square': [200, 200], 'half-letter': [140, 216],
  }
  const [w, h] = dims[paper]
  const scale  = 460 / h
  const dW     = Math.round(w * scale)
  const dH     = Math.round(h * scale)

  const captionText = page.captionOverride
    ?? [caption?.title, caption?.description, caption?.location].filter(Boolean).join(' — ')

  const root: React.CSSProperties = { width: dW, height: dH, position: 'relative', background: '#fff', userSelect: 'none' }

  return (
    <div style={root} className="rounded shadow-2xl overflow-hidden border border-border shrink-0">
      {page.template === 'fullBleed' && (
        <>
          {thumb
            ? <img src={thumb} draggable={false} style={{ position: 'absolute', inset: 0, width: '100%', height: '100%', objectFit: 'cover' }} alt="" />
            : <div style={{ position: 'absolute', inset: 0, background: '#1a1a1a', display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
                <ImageIcon size={32} color="#444" />
              </div>
          }
          {captionText && (
            <div style={{ position: 'absolute', bottom: 0, inset: 'auto 0 0', background: 'rgba(245,245,245,0.93)', padding: '6px 10px', fontSize: 10, color: '#222', lineHeight: 1.4 }}>
              {captionText}
            </div>
          )}
        </>
      )}

      {page.template === 'top75' && (
        <>
          <div style={{ position: 'absolute', top: 0, left: 0, right: 0, height: '75%', overflow: 'hidden', background: '#222' }}>
            {thumb && <img src={thumb} draggable={false} style={{ width: '100%', height: '100%', objectFit: 'cover' }} alt="" />}
          </div>
          <div style={{ position: 'absolute', bottom: 0, left: 0, right: 0, height: '25%', background: '#f5f5f5', padding: '6px 8px', overflow: 'hidden' }}>
            {caption?.title && <div style={{ fontSize: 10, fontWeight: 700, color: '#111', marginBottom: 2, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{caption.title}</div>}
            {caption?.description && <div style={{ fontSize: 8, color: '#555', lineHeight: 1.35, overflow: 'hidden', display: '-webkit-box', WebkitLineClamp: 3, WebkitBoxOrient: 'vertical' }}>{caption.description}</div>}
          </div>
        </>
      )}

      {(page.template === 'left50' || page.template === 'right50') && (() => {
        const imgSide  = page.template === 'left50' ? 'left'  : 'right'
        const textSide = page.template === 'left50' ? 'right' : 'left'
        return (
          <>
            <div style={{ position: 'absolute', top: 0, bottom: 0, [imgSide]: 0, width: '50%', background: '#222', overflow: 'hidden' }}>
              {thumb && <img src={thumb} draggable={false} style={{ width: '100%', height: '100%', objectFit: 'cover' }} alt="" />}
            </div>
            <div style={{ position: 'absolute', top: 0, bottom: 0, [textSide]: 0, width: '50%', background: '#f5f5f5', padding: '14px 10px', overflow: 'hidden' }}>
              {caption?.title && <div style={{ fontSize: 11, fontWeight: 700, color: '#111', marginBottom: 6, lineHeight: 1.3 }}>{caption.title}</div>}
              {caption?.description && <div style={{ fontSize: 8, color: '#444', lineHeight: 1.45 }}>{caption.description}</div>}
              {caption?.location && <div style={{ fontSize: 8, color: '#888', marginTop: 6, fontStyle: 'italic' }}>{caption.location}</div>}
            </div>
          </>
        )
      })()}

      {page.template === 'gridTwo' && (() => {
        const slots = [thumb, t(page.imageId2)]
        return (
          <div style={{ display: 'flex', width: '100%', height: '100%' }}>
            {slots.map((src, i) => (
              <div key={i} style={{ flex: 1, overflow: 'hidden', background: '#222', borderRight: i === 0 ? '2px solid #fff' : undefined, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
                {src
                  ? <img src={src} draggable={false} style={{ width: '100%', height: '100%', objectFit: 'cover' }} alt="" />
                  : <ImageIcon size={24} color="#555" />}
              </div>
            ))}
          </div>
        )
      })()}

      {page.template === 'titlePage' && (
        <div style={{ display: 'flex', flexDirection: 'column', justifyContent: 'center', padding: '20px 18px', height: '100%', background: '#fafafa' }}>
          {caption?.title && <div style={{ fontSize: 18, fontWeight: 800, color: '#111', marginBottom: 10, lineHeight: 1.2 }}>{caption.title}</div>}
          {caption?.description && <div style={{ fontSize: 9, color: '#555', lineHeight: 1.55 }}>{caption.description}</div>}
        </div>
      )}

      {page.template === 'pullQuote' && (
        <>
          {thumb
            ? <img src={thumb} draggable={false} style={{ position: 'absolute', inset: 0, width: '100%', height: '100%', objectFit: 'cover' }} alt="" />
            : <div style={{ position: 'absolute', inset: 0, background: '#1a1a1a' }} />
          }
          <div style={{ position: 'absolute', bottom: 0, left: 0, right: 0, height: '38%', background: 'rgba(10,10,10,0.78)', padding: '10px 12px', display: 'flex', flexDirection: 'column', justifyContent: 'center' }}>
            {caption?.title && <div style={{ fontSize: 13, fontWeight: 700, fontStyle: 'italic', color: '#fff', lineHeight: 1.25, marginBottom: 4, overflow: 'hidden', display: '-webkit-box', WebkitLineClamp: 2, WebkitBoxOrient: 'vertical' }}>{caption.title}</div>}
            {caption?.description && <div style={{ fontSize: 8, color: 'rgba(255,255,255,0.7)', lineHeight: 1.4, overflow: 'hidden', display: '-webkit-box', WebkitLineClamp: 2, WebkitBoxOrient: 'vertical' }}>{caption.description}</div>}
          </div>
        </>
      )}

      {page.template === 'imageLead' && (
        <>
          <div style={{ position: 'absolute', top: 0, left: 0, right: 0, height: '60%', background: '#222', overflow: 'hidden' }}>
            {thumb && <img src={thumb} draggable={false} style={{ width: '100%', height: '100%', objectFit: 'cover' }} alt="" />}
          </div>
          <div style={{ position: 'absolute', bottom: 0, left: 0, right: 0, height: '40%', background: '#fff', padding: '10px 12px', overflow: 'hidden' }}>
            {caption?.title && <div style={{ fontSize: 13, fontWeight: 800, color: '#111', lineHeight: 1.2, marginBottom: 4, overflow: 'hidden', display: '-webkit-box', WebkitLineClamp: 2, WebkitBoxOrient: 'vertical' }}>{caption.title}</div>}
            {caption?.description && <div style={{ fontSize: 8, color: '#444', lineHeight: 1.4, overflow: 'hidden', display: '-webkit-box', WebkitLineClamp: 3, WebkitBoxOrient: 'vertical' }}>{caption.description}</div>}
          </div>
        </>
      )}

      {page.template === 'panoramic' && (
        <div style={{ width: '100%', height: '100%', background: '#fff', display: 'flex', flexDirection: 'column', padding: '12px 10px' }}>
          <div style={{ flex: '0 0 72%', overflow: 'hidden', display: 'flex', alignItems: 'center', justifyContent: 'center', background: '#f0f0f0' }}>
            {thumb
              ? <img src={thumb} draggable={false} style={{ maxWidth: '100%', maxHeight: '100%', objectFit: 'contain' }} alt="" />
              : <ImageIcon size={28} color="#bbb" />
            }
          </div>
          <div style={{ flex: 1, paddingTop: 8, overflow: 'hidden' }}>
            {caption?.title && <div style={{ fontSize: 10, fontWeight: 700, color: '#111', marginBottom: 2, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{caption.title}</div>}
            {caption?.description && <div style={{ fontSize: 8, color: '#555', lineHeight: 1.35, overflow: 'hidden', display: '-webkit-box', WebkitLineClamp: 2, WebkitBoxOrient: 'vertical' }}>{caption.description}</div>}
          </div>
        </div>
      )}

      {page.template === 'editorial' && (
        <>
          <div style={{ position: 'absolute', top: 0, left: 0, bottom: 0, width: '65%', background: '#222', overflow: 'hidden' }}>
            {thumb && <img src={thumb} draggable={false} style={{ width: '100%', height: '100%', objectFit: 'cover' }} alt="" />}
          </div>
          <div style={{ position: 'absolute', top: 0, right: 0, bottom: 0, width: '35%', background: '#fafafa', padding: '14px 8px', overflow: 'hidden' }}>
            {caption?.title && <div style={{ fontSize: 10, fontWeight: 700, color: '#111', marginBottom: 6, lineHeight: 1.3, overflow: 'hidden', display: '-webkit-box', WebkitLineClamp: 3, WebkitBoxOrient: 'vertical' }}>{caption.title}</div>}
            {caption?.description && <div style={{ fontSize: 7, color: '#444', lineHeight: 1.55, overflow: 'hidden', display: '-webkit-box', WebkitLineClamp: 8, WebkitBoxOrient: 'vertical' }}>{caption.description}</div>}
            {caption?.location && <div style={{ position: 'absolute', bottom: 10, left: 8, right: 8, fontSize: 7, color: '#999', fontStyle: 'italic', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{caption.location}</div>}
          </div>
        </>
      )}

      {page.template === 'triptych' && (() => {
        const slots = [t(page.imageId), t(page.imageId2), t(page.imageId3)]
        return (
          <div style={{ display: 'flex', width: '100%', height: '100%' }}>
            {slots.map((src, i) => (
              <div key={i} style={{ flex: 1, background: '#222', borderRight: i < 2 ? '1px solid #fff' : undefined, overflow: 'hidden' }}>
                {src && <img src={src} draggable={false} style={{ width: '100%', height: '100%', objectFit: 'cover' }} alt="" />}
              </div>
            ))}
          </div>
        )
      })()}

      {page.template === 'grid4x' && (
        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gridTemplateRows: '1fr 1fr', width: '100%', height: '100%', gap: 1, background: '#fff' }}>
          {[t(page.imageId), t(page.imageId2), t(page.imageId3), t(page.imageId4)].map((src, i) => (
            <div key={i} style={{ background: '#222', overflow: 'hidden' }}>
              {src && <img src={src} draggable={false} style={{ width: '100%', height: '100%', objectFit: 'cover' }} alt="" />}
            </div>
          ))}
        </div>
      )}

      {page.template === 'photoEssay' && (
        <div style={{ width: '100%', height: '100%', display: 'flex', gap: 1 }}>
          <div style={{ flex: '0 0 50%', background: '#222', overflow: 'hidden', position: 'relative' }}>
            {thumb && <img src={thumb} draggable={false} style={{ width: '100%', height: '100%', objectFit: 'cover' }} alt="" />}
            {caption?.title && (
              <div style={{ position: 'absolute', bottom: 8, left: 8, right: 8, fontSize: 9, fontWeight: 700, color: '#fff', textShadow: '0 1px 3px rgba(0,0,0,0.7)', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{caption.title}</div>
            )}
          </div>
          <div style={{ flex: 1, display: 'grid', gridTemplateColumns: '1fr 1fr', gridTemplateRows: '1fr 1fr', gap: 1 }}>
            {[t(page.imageId2), t(page.imageId3), t(page.imageId4), t(page.imageId5)].map((src, i) => (
              <div key={i} style={{ background: '#2a2a2a', overflow: 'hidden', display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
                {src
                  ? <img src={src} draggable={false} style={{ width: '100%', height: '100%', objectFit: 'cover' }} alt="" />
                  : <ImageIcon size={10} color="#444" />}
              </div>
            ))}
          </div>
        </div>
      )}

      {page.template === 'contactSheet' && (() => {
        const slotSrcs = [
          t(page.imageId),  t(page.imageId2), t(page.imageId3),
          t(page.imageId4), t(page.imageId5), t(page.imageId6),
          t(page.imageId7), t(page.imageId8), t(page.imageId9),
        ]
        return (
          <div style={{ width: '100%', height: '100%', background: '#e8e8e8', display: 'grid', gridTemplateColumns: '1fr 1fr 1fr', gridTemplateRows: '1fr 1fr 1fr', gap: 1.5, padding: 4 }}>
            {slotSrcs.map((src, i) => (
              <div key={i} style={{ background: '#bbb', overflow: 'hidden', display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
                {src
                  ? <img src={src} draggable={false} style={{ width: '100%', height: '100%', objectFit: 'cover' }} alt="" />
                  : <ImageIcon size={8} color="#999" />}
              </div>
            ))}
          </div>
        )
      })()}

      {page.template === 'featureSpread' && (
        <div style={{ width: '100%', height: '100%', position: 'relative' }}>
          <div style={{ position: 'absolute', top: 0, left: 0, right: 0, height: '30%', display: 'flex', gap: 1 }}>
            {[t(page.imageId2), t(page.imageId3)].map((src, i) => (
              <div key={i} style={{ flex: 1, background: '#2a2a2a', overflow: 'hidden' }}>
                {src && <img src={src} draggable={false} style={{ width: '100%', height: '100%', objectFit: 'cover' }} alt="" />}
              </div>
            ))}
          </div>
          <div style={{ position: 'absolute', top: 'calc(30% + 1px)', left: 0, right: 0, bottom: 0, background: '#222', overflow: 'hidden' }}>
            {thumb && <img src={thumb} draggable={false} style={{ width: '100%', height: '100%', objectFit: 'cover' }} alt="" />}
            {caption?.title && (
              <div style={{ position: 'absolute', bottom: 10, left: 10, right: 10, fontSize: 11, fontWeight: 800, color: '#fff', lineHeight: 1.2, overflow: 'hidden', display: '-webkit-box', WebkitLineClamp: 2, WebkitBoxOrient: 'vertical', textShadow: '0 1px 4px rgba(0,0,0,0.6)' }}>{caption.title}</div>
            )}
          </div>
          <div style={{ position: 'absolute', top: '30%', left: 0, right: 0, height: 1, background: '#fff' }} />
        </div>
      )}
    </div>
  )
}
