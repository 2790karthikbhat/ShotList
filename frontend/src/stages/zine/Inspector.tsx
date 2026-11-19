import type { Caption, SequenceSuggestion, TemplateId, ZinePage } from '../../types'
import { TEMPLATE_LABELS } from '../../types'
import { TemplateIcon } from './TemplateIcon'
import { IconSparkle, IconPhoto } from '../../components/icons'

type PickerMode = null | { mode: 'add' } | { mode: 'swap'; slot: 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 }

export function Inspector({
  currentPage,
  pagesEmpty,
  currentCap,
  currentThumb,
  suggestions,
  setPickerMode,
  onClearPageImage,
  onTemplate,
  editingOvr,
  setEditingOvr,
  overrideText,
  setOverrideText,
  saveOverride,
  clearOverride,
}: {
  currentPage: ZinePage | null
  pagesEmpty: boolean
  currentCap: Caption | undefined
  currentThumb: string | undefined
  suggestions: Array<SequenceSuggestion | null>
  setPickerMode: (mode: PickerMode) => void
  onClearPageImage: () => void
  onTemplate: (template: TemplateId) => void
  editingOvr: boolean
  setEditingOvr: (v: boolean) => void
  overrideText: string
  setOverrideText: (v: string) => void
  saveOverride: () => void
  clearOverride: () => void
}) {
  return (
    <div className="w-64 shrink-0 border-l border-border vibrancy-panel overflow-y-auto flex flex-col gap-5 p-3.5">
      {currentPage ? (
        <>
          {/* Image section */}
          <div>
            <div className="text-[10px] font-semibold text-lo uppercase tracking-widest mb-2">Image</div>

            {currentPage.imageId && currentThumb ? (
              <div className="relative group/img">
                <img
                  src={currentThumb}
                  alt=""
                  className="w-full aspect-video object-cover rounded-lg"
                  draggable={false}
                />
                <div className="absolute inset-0 bg-black/0 group-hover/img:bg-black/40 rounded-lg transition-all flex items-center justify-center gap-2 opacity-0 group-hover/img:opacity-100">
                  <button
                    onClick={() => setPickerMode({ mode: 'swap', slot: 1 })}
                    className="text-[10px] px-2 py-1 bg-black/80 rounded text-white hover:bg-amber-700"
                  >
                    Change
                  </button>
                  <button
                    onClick={onClearPageImage}
                    className="text-[10px] px-2 py-1 bg-black/80 rounded text-white hover:bg-red-700"
                  >
                    Remove
                  </button>
                </div>
              </div>
            ) : (
              <button
                onClick={() => setPickerMode({ mode: 'swap', slot: 1 })}
                className="w-full aspect-video bg-surface rounded-lg border-2 border-dashed border-border
                  hover:border-amber-500/50 flex flex-col items-center justify-center gap-1
                  text-lo hover:text-amber-400 transition-all"
              >
                <IconPhoto size={18} />
                <span className="text-[10px]">Set image</span>
              </button>
            )}

            {/* Extra slots for multi-image templates */}
            {(['gridTwo', 'triptych', 'grid4x', 'featureSpread', 'editorial', 'photoEssay', 'contactSheet'] as TemplateId[]).includes(currentPage.template) && (() => {
              const slotCount = currentPage.template === 'gridTwo' ? 2
                : currentPage.template === 'triptych' ? 3
                : currentPage.template === 'grid4x' ? 4
                : currentPage.template === 'featureSpread' ? 3
                : currentPage.template === 'photoEssay' ? 5
                : currentPage.template === 'contactSheet' ? 9
                : 1
              const slotLabels: Record<string, string[]> = {
                gridTwo:       ['Left', 'Right'],
                triptych:      ['Left', 'Centre', 'Right'],
                grid4x:        ['Top-left', 'Top-right', 'Bot-left', 'Bot-right'],
                featureSpread: ['Hero', 'Bottom-left', 'Bottom-right'],
                editorial:     ['Main'],
                photoEssay:    ['Main', 'TR', 'BR', 'TL2', 'BL2'],
                contactSheet:  ['1', '2', '3', '4', '5', '6', '7', '8', '9'],
              }
              const slotIds = [
                currentPage.imageId,  currentPage.imageId2, currentPage.imageId3,
                currentPage.imageId4, currentPage.imageId5, currentPage.imageId6,
                currentPage.imageId7, currentPage.imageId8, currentPage.imageId9,
              ]
              return (
                <div className="mt-2 grid grid-cols-2 gap-1">
                  {Array.from({ length: slotCount }, (_, i) => {
                    const slot = (i + 1) as 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9
                    const label = slotLabels[currentPage.template]?.[i] ?? `Slot ${slot}`
                    const hasImg = !!slotIds[i]
                    return (
                      <button
                        key={slot}
                        onClick={() => setPickerMode({ mode: 'swap', slot })}
                        className={`text-[9px] px-2 py-1.5 rounded border flex items-center gap-1 transition-all
                          ${hasImg
                            ? 'border-lo text-mid hover:border-amber-500'
                            : 'border-dashed border-border text-lo hover:border-amber-500/50 hover:text-amber-400'}`}
                      >
                        <IconPhoto size={10} />
                        {label}
                      </button>
                    )
                  })}
                </div>
              )
            })()}
          </div>

          {/* Layout (visual template picker) */}
          <div>
            <div className="text-[10px] font-semibold text-lo uppercase tracking-widest mb-2">Layout</div>
            <div className="grid grid-cols-2 gap-1.5">
              {(Object.keys(TEMPLATE_LABELS) as TemplateId[]).map(t => (
                <button
                  key={t}
                  onClick={() => onTemplate(t)}
                  title={TEMPLATE_LABELS[t]}
                  className={`flex flex-col items-center gap-1.5 p-2.5 rounded-xl border transition-all
                    ${currentPage.template === t
                      ? 'border-amber-500 bg-amber-950/40 text-amber-300'
                      : 'border-border hover:border-mid text-mid hover:text-hi bg-surface/40'}`}
                >
                  <div className={currentPage.template === t ? 'text-amber-400' : 'text-lo'}>
                    <TemplateIcon id={t} size={64} />
                  </div>
                  <span className="text-[9px] leading-tight text-center">{TEMPLATE_LABELS[t]}</span>
                </button>
              ))}
            </div>
          </div>

          {/* Caption override */}
          <div>
            <div className="text-[10px] font-semibold text-lo uppercase tracking-widest mb-2">Caption Override</div>
            {editingOvr ? (
              <div className="flex flex-col gap-1.5">
                <textarea
                  value={overrideText}
                  onChange={e => setOverrideText(e.target.value)}
                  rows={4}
                  placeholder="Custom text…"
                  autoFocus
                  className="w-full bg-surface border border-border rounded text-xs text-hi p-2 resize-none focus:outline-none focus:border-amber-500"
                />
                <div className="flex gap-1.5">
                  <button onClick={saveOverride} className="flex-1 text-xs px-2 py-1 bg-amber-600 hover:bg-amber-500 rounded text-white">Save</button>
                  <button onClick={() => { setEditingOvr(false); setOverrideText(currentPage.captionOverride ?? '') }}
                    className="flex-1 text-xs px-2 py-1 bg-surface hover:bg-lo/20 rounded text-hi">Cancel</button>
                </div>
              </div>
            ) : (
              <div
                onClick={() => setEditingOvr(true)}
                className="text-xs text-lo bg-surface rounded p-2 min-h-[44px] cursor-pointer hover:bg-lo/20 transition-colors"
              >
                {currentPage.captionOverride
                  ? <span className="text-hi whitespace-pre-wrap">{currentPage.captionOverride}</span>
                  : <span className="italic">Click to override caption…</span>}
              </div>
            )}
            {currentPage.captionOverride && !editingOvr && (
              <button onClick={clearOverride} className="mt-1 text-[10px] text-lo hover:text-red-400 transition-colors">
                Clear override
              </button>
            )}
          </div>

          {/* Auto caption preview */}
          {currentCap && !currentPage.captionOverride && (
            <div>
              <div className="text-[10px] font-semibold text-lo uppercase tracking-widest mb-2">Caption</div>
              {currentCap.title && <div className="text-xs font-semibold text-hi mb-1 leading-tight">{currentCap.title}</div>}
              {currentCap.description && <div className="text-[11px] text-mid leading-relaxed">{currentCap.description}</div>}
              {currentCap.location && <div className="text-[10px] text-lo mt-1 italic">{currentCap.location}</div>}
            </div>
          )}

          {/* AI suggestions */}
          {suggestions.length > 0 && (
            <div>
              <div className="text-[10px] font-semibold text-violet-400 uppercase tracking-widest mb-2 flex items-center gap-1">
                <IconSparkle size={10} /> Sequence
              </div>
              <div className="flex flex-col gap-1.5">
                {suggestions.slice(0, 5).filter((s): s is SequenceSuggestion => s !== null).map((s, i) => (
                  <div key={i} className="text-[10px] bg-surface rounded p-2 leading-relaxed">
                    <span className="text-violet-300 font-medium">pos {s.position + 1}:</span>
                    <span className="ml-1 text-mid">{s.reason}</span>
                  </div>
                ))}
              </div>
            </div>
          )}
        </>
      ) : (
        <div className="text-xs text-lo text-center pt-6 italic">
          {pagesEmpty ? 'Add pages using the + buttons in the strip below.' : 'Select a page to inspect.'}
        </div>
      )}
    </div>
  )
}
