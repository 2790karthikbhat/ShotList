import type { ImageInfo } from '../../types'
import { IconX, IconPhoto } from '../../components/icons'

export function ImagePicker({
  images, thumbnails, usedImageIds, onPick, onClose,
}: {
  images:       Record<string, ImageInfo>
  thumbnails:   Record<string, string>
  usedImageIds: Set<string>
  onPick:       (imageId: string) => void
  onClose:      () => void
}) {
  const all = Object.values(images)

  return (
    <div className="fixed inset-0 z-50 bg-black/70 flex items-center justify-center p-6" onClick={onClose}>
      <div
        onClick={e => e.stopPropagation()}
        className="vibrancy-panel border border-border rounded-2xl w-[600px] max-h-[70vh] flex flex-col shadow-panel"
      >
        <div className="flex items-center justify-between px-4 py-3 border-b border-border">
          <span className="text-sm font-semibold text-hi">Choose image</span>
          <button onClick={onClose} className="text-lo hover:text-hi">
            <IconX size={16} />
          </button>
        </div>

        <div className="overflow-y-auto p-3">
          {all.length === 0 && (
            <p className="text-sm text-lo text-center py-8">No images in session.</p>
          )}
          <div className="grid grid-cols-5 gap-2">
            {all.map(img => {
              const thumb  = thumbnails[img.id]
              const inUse  = usedImageIds.has(img.id)
              return (
                <button
                  key={img.id}
                  onClick={() => onPick(img.id)}
                  className={`relative aspect-[3/2] rounded overflow-hidden border-2 transition-all
                    ${inUse
                      ? 'border-amber-500/60 opacity-60 hover:opacity-90'
                      : 'border-transparent hover:border-amber-400'}`}
                  title={img.filename + (inUse ? ' (already in zine)' : '')}
                >
                  {thumb
                    ? <img src={thumb} alt={img.filename} className="w-full h-full object-cover" draggable={false} />
                    : <div className="w-full h-full bg-surface flex items-center justify-center">
                        <IconPhoto size={14} className="text-lo" />
                      </div>
                  }
                  {inUse && (
                    <div className="absolute inset-0 flex items-center justify-center bg-black/40">
                      <span className="text-[8px] text-amber-300 font-medium bg-black/60 rounded px-1">In zine</span>
                    </div>
                  )}
                </button>
              )
            })}
          </div>
        </div>

        <div className="px-4 py-3 border-t border-border text-xs text-lo">
          {all.length} images · click to add
        </div>
      </div>
    </div>
  )
}
