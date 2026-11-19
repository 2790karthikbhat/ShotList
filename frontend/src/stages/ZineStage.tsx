// frontend/src/stages/ZineStage.tsx

import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import {
  DndContext, closestCenter, DragEndEvent,
  PointerSensor, useSensor, useSensors,
} from '@dnd-kit/core'
import {
  SortableContext, horizontalListSortingStrategy,
  arrayMove,
} from '@dnd-kit/sortable'
import { listen } from '@tauri-apps/api/event'

import {
  createOrGetZine, getCaption, addZinePage, removeZinePage,
  setPageImage, updatePageOrder, setPageTemplate,
  setPageCaptionOverride, setZinePaperSize, exportPdf, aiSequence,
  getZineThumbUrls,
} from '../lib/ipc'
import { useStore } from '../store'
import { StageToolbar } from '../components/StageToolbar'
import {
  IconGrid, IconFileDown, IconSparkle, IconChevronLeft, IconChevronRight,
  IconLoader, IconPlus, IconSettings,
} from '../components/icons'
import type {
  Zine, TemplateId, PaperSize, Caption,
  SequenceSuggestion, PdfExportProgress,
} from '../types'
import { DEFAULT_CAPTION, PAPER_SIZE_LABELS } from '../types'
import { Inspector } from './zine/Inspector'
import { ImagePicker } from './zine/ImagePicker'
import { PagePreview } from './zine/PagePreview'
import { SortablePage } from './zine/SortablePage'

// ── Main stage ────────────────────────────────────────────────────────────────

export function ZineStage() {
  const storeThumbnails = useStore(s => s.thumbnails)
  const images          = useStore(s => s.images)
  const showToast       = useStore(s => s.showToast)
  const openSettings    = useStore(s => s.openSettings)

  // Thumbnails loaded directly from disk cache — reliable across app restarts
  // because the cache is keyed by the same stable image IDs stored in zine_pages.
  const [cachedThumbs, setCachedThumbs] = useState<Record<string, string>>({})
  // Merge: store takes priority (it's the live `thumbnail-ready` feed, so it
  // reflects the latest saved rotate/crop); cache fills in ids the store
  // hasn't seen yet — e.g. right after an app restart, before anything has
  // been re-decoded.
  const thumbnails = useMemo(
    () => ({ ...cachedThumbs, ...storeThumbnails }),
    [cachedThumbs, storeThumbnails],
  )

  const [zine,         setZine]         = useState<Zine | null>(null)
  const [captions,     setCaptions]     = useState<Record<string, Caption>>({})
  const [selectedIdx,  setSelectedIdx]  = useState(0)
  const [loading,      setLoading]      = useState(true)
  const [exporting,    setExporting]    = useState(false)
  const [sequencing,   setSequencing]   = useState(false)
  const [exportMsg,    setExportMsg]    = useState('')
  const [suggestions,  setSuggestions]  = useState<Array<SequenceSuggestion | null>>([])
  const [overrideText, setOverrideText] = useState('')
  const [editingOvr,   setEditingOvr]   = useState(false)
  // picker state: 'add' = adding new page, 'swap' = changing image of current page
  const [pickerMode,   setPickerMode]   = useState<null | { mode: 'add' } | { mode: 'swap'; slot: 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 }>(null)

  const unsubRef = useRef<Array<() => void>>([])

  const sensors = useSensors(useSensor(PointerSensor, { activationConstraint: { distance: 5 } }))

  // ── Load zine + captions ───────────────────────────────────────────────────

  const loadZine = useCallback(async () => {
    setLoading(true)
    try {
      const z = await createOrGetZine()
      setZine(z)
      // Collect every image ID across all slots for thumbnail + caption loading.
      const allSlotIds = (p: typeof z.pages[0]) => [
        p.imageId, p.imageId2, p.imageId3, p.imageId4, p.imageId5,
        p.imageId6, p.imageId7, p.imageId8, p.imageId9,
      ].filter(Boolean) as string[]
      const ids = [...new Set(z.pages.flatMap(allSlotIds))]
      if (ids.length > 0) {
        // Load thumbnails directly from the on-disk cache — this works even when
        // in-memory ImageIds don't match (e.g. after an app restart) because the
        // cache files are named by the same stable IDs stored in zine_pages.
        getZineThumbUrls(ids).then(setCachedThumbs).catch(() => {})
      }
      // Only load captions for slot-1 images (they carry the page caption).
      const captionIds = [...new Set(z.pages.flatMap(p => p.imageId ? [p.imageId] : []))]
      const entries = await Promise.all(
        captionIds.map(async id => {
          try { return [id, await getCaption(id)] as const }
          catch { return [id, DEFAULT_CAPTION] as const }
        })
      )
      setCaptions(Object.fromEntries(entries))
    } catch (e) { console.error(e) }
    finally { setLoading(false) }
  }, [])

  useEffect(() => { loadZine() }, [loadZine])

  // Load caption + thumbnail for a newly assigned image
  const ensureCaption = useCallback(async (imageId: string) => {
    if (!captions[imageId]) {
      try {
        const c = await getCaption(imageId)
        setCaptions(prev => ({ ...prev, [imageId]: c }))
      } catch {
        // best-effort — leave the caption unset if the load fails
      }
    }
    if (!thumbnails[imageId]) {
      getZineThumbUrls([imageId]).then(m => {
        if (m[imageId]) setCachedThumbs(prev => ({ ...prev, [imageId]: m[imageId] }))
      }).catch(() => {})
    }
  }, [captions, thumbnails])

  // ── Event listeners ────────────────────────────────────────────────────────

  useEffect(() => {
    const subs: Array<() => void> = []

    listen<SequenceSuggestion>('sequence-suggestion', ev => {
      setSuggestions(prev => {
        const next = [...prev]
        while (next.length <= ev.payload.position) next.push(null)
        next[ev.payload.position] = ev.payload
        return next
      })
    }).then(u => subs.push(u))

    listen<PdfExportProgress>('pdf-export-progress', ev => {
      setExportMsg(ev.payload.message)
    }).then(u => subs.push(u))

    unsubRef.current = subs
    return () => { subs.forEach(u => u()); unsubRef.current = [] }
  }, [])

  // Sync override text on selection change
  useEffect(() => {
    if (!zine) return
    const p = zine.pages[selectedIdx]
    setOverrideText(p?.captionOverride ?? '')
    setEditingOvr(false)
  }, [selectedIdx, zine])

  // ── Derived ────────────────────────────────────────────────────────────────

  const pages        = zine?.pages ?? []
  const currentPage  = pages[selectedIdx] ?? null
  const currentCap   = currentPage?.imageId ? captions[currentPage.imageId] : undefined
  const currentThumb = currentPage?.imageId ? thumbnails[currentPage.imageId] : undefined
  const usedImageIds = new Set(pages.flatMap(p => p.imageId ? [p.imageId] : []))

  // ── Handlers ───────────────────────────────────────────────────────────────

  const onDragEnd = useCallback(async (event: DragEndEvent) => {
    const { active, over } = event
    if (!over || active.id === over.id || !zine) return
    const oldIdx   = zine.pages.findIndex(p => p.id === active.id)
    const newIdx   = zine.pages.findIndex(p => p.id === over.id)
    if (oldIdx === -1 || newIdx === -1) return
    const reordered = arrayMove(zine.pages, oldIdx, newIdx).map((p, i) => ({ ...p, position: i }))
    setZine({ ...zine, pages: reordered })
    setSelectedIdx(newIdx)
    await updatePageOrder(reordered.map(p => p.id)).catch(console.error)
  }, [zine])

  const onRemovePage = useCallback(async (pageId: string) => {
    if (!zine) return
    const newPages = zine.pages.filter(p => p.id !== pageId).map((p, i) => ({ ...p, position: i }))
    setZine({ ...zine, pages: newPages })
    setSelectedIdx(i => Math.min(i, Math.max(0, newPages.length - 1)))
    await removeZinePage(pageId).catch(console.error)
  }, [zine])

  const onTemplate = useCallback(async (template: TemplateId) => {
    if (!currentPage || !zine) return
    setZine({ ...zine, pages: zine.pages.map(p => p.id === currentPage.id ? { ...p, template } : p) })
    await setPageTemplate(currentPage.id, template).catch(console.error)
  }, [currentPage, zine])

  // Image picker: pick an image (add new page or swap existing page image)
  const onPickImage = useCallback(async (imageId: string) => {
    const pm = pickerMode
    setPickerMode(null)
    if (!zine || !pm) return
    await ensureCaption(imageId)

    if (pm.mode === 'add') {
      try {
        const page = await addZinePage(zine.id, imageId, 'top75')
        setZine(prev => prev ? { ...prev, pages: [...prev.pages, { ...page, position: prev.pages.length }] } : prev)
        setSelectedIdx(zine.pages.length)
      } catch (e) { console.error(e) }
    } else if (pm.mode === 'swap' && currentPage) {
      const slot = pm.slot
      const slotKey = (['imageId','imageId2','imageId3','imageId4','imageId5','imageId6','imageId7','imageId8','imageId9'] as const)[slot - 1]
      setZine({ ...zine, pages: zine.pages.map(p => p.id === currentPage.id ? { ...p, [slotKey]: imageId } : p) })
      await setPageImage(currentPage.id, imageId, slot).catch(console.error)
    }
  }, [zine, pickerMode, currentPage, ensureCaption])

  const onAddBlankPage = useCallback(async () => {
    if (!zine) return
    try {
      const page = await addZinePage(zine.id, null, 'titlePage')
      setZine(prev => prev ? { ...prev, pages: [...prev.pages, { ...page, position: prev.pages.length }] } : prev)
      setSelectedIdx(zine.pages.length)
    } catch (e) { console.error(e) }
  }, [zine])

  const onClearPageImage = useCallback(async () => {
    if (!currentPage || !zine) return
    setZine({ ...zine, pages: zine.pages.map(p => p.id === currentPage.id ? { ...p, imageId: null } : p) })
    await setPageImage(currentPage.id, null).catch(console.error)
  }, [currentPage, zine])

  const saveOverride = useCallback(async () => {
    if (!currentPage || !zine) return
    const val = overrideText.trim() || null
    setZine({ ...zine, pages: zine.pages.map(p => p.id === currentPage.id ? { ...p, captionOverride: val } : p) })
    setEditingOvr(false)
    await setPageCaptionOverride(currentPage.id, val).catch(console.error)
  }, [currentPage, zine, overrideText])

  const clearOverride = useCallback(async () => {
    if (!currentPage || !zine) return
    setZine({ ...zine, pages: zine.pages.map(p => p.id === currentPage.id ? { ...p, captionOverride: null } : p) })
    await setPageCaptionOverride(currentPage.id, null).catch(console.error)
  }, [currentPage, zine])

  const onPaperSize = useCallback(async (size: PaperSize) => {
    if (!zine) return
    setZine({ ...zine, paperSize: size })
    await setZinePaperSize(zine.id, size).catch(console.error)
  }, [zine])

  const onAiSequence = useCallback(async () => {
    setSuggestions([])
    setSequencing(true)
    try {
      const msg = await aiSequence()
      await loadZine()
      setSelectedIdx(0)
      showToast(msg, msg.includes('could not') ? 'error' : 'success')
    } catch (e) {
      showToast(String(e), 'error')
    } finally { setSequencing(false) }
  }, [loadZine, showToast])

  const onExportPdf = useCallback(async () => {
    setExporting(true)
    setExportMsg('Starting…')
    try {
      const msg = await exportPdf(92)
      setExportMsg(msg)
    } catch (e) {
      setExportMsg(String(e))
    } finally { setExporting(false) }
  }, [])

  // ── Guards ─────────────────────────────────────────────────────────────────

  if (loading) return (
    <div className="flex-1 flex items-center justify-center text-lo gap-2">
      <IconLoader size={16} className="animate-spin" /> Building zine…
    </div>
  )
  if (!zine) return (
    <div className="flex-1 flex items-center justify-center text-lo">No session open.</div>
  )

  // ── Render ─────────────────────────────────────────────────────────────────

  return (
    <div className="flex flex-col h-full overflow-hidden bg-bg">

      {/* ── Toolbar ─────────────────────────────────────────────────────────── */}
      <StageToolbar
        icon={IconGrid}
        stageLabel="Zine"
        right={
          <div className="flex items-center gap-2">
            <button
              onClick={onAiSequence} disabled={sequencing || pages.length === 0}
              className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium
                bg-violet-900 hover:bg-violet-800 text-violet-200 disabled:opacity-40 transition-colors"
            >
              {sequencing ? <IconLoader size={14} className="animate-spin" /> : <IconSparkle size={14} />}
              {sequencing ? 'Sequencing…' : 'AI Sequence'}
            </button>

            <button
              onClick={onExportPdf} disabled={exporting || pages.length === 0}
              title={exporting ? exportMsg : 'Export PDF to Zine/zine.pdf'}
              className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium
                bg-amber-700 hover:bg-amber-600 text-amber-100 disabled:opacity-40 transition-colors"
            >
              {exporting ? <IconLoader size={14} className="animate-spin" /> : <IconFileDown size={14} />}
              {exporting ? (exportMsg.length > 28 ? exportMsg.slice(0, 26) + '…' : exportMsg) : 'Export PDF'}
            </button>

            <button
              onClick={openSettings}
              className="w-[36px] h-[36px] flex items-center justify-center rounded-lg text-lo hover:text-mid hover:bg-surface text-2xl transition-colors"
            >
              <IconSettings />
            </button>
          </div>
        }
      >
        <select
          value={zine.paperSize}
          onChange={e => onPaperSize(e.target.value as PaperSize)}
          className="bg-surface text-hi text-xs rounded-lg px-2.5 py-1.5 border border-border"
        >
          {(Object.keys(PAPER_SIZE_LABELS) as PaperSize[]).map(s => (
            <option key={s} value={s}>{PAPER_SIZE_LABELS[s]}</option>
          ))}
        </select>

        <span className="text-lo text-xs">{pages.length} pages</span>
      </StageToolbar>

      {/* ── Body ────────────────────────────────────────────────────────────── */}
      <div className="flex flex-1 min-h-0">

        {/* Centre: preview + strip */}
        <div className="flex flex-col flex-1 min-w-0">

          {/* Large preview */}
          <div className="flex-1 flex items-center justify-center bg-bg relative overflow-hidden p-6">
            <button
              onClick={() => setSelectedIdx(i => Math.max(0, i - 1))}
              disabled={selectedIdx === 0}
              className="absolute left-4 top-1/2 -translate-y-1/2 p-2 rounded-full
                bg-surface/80 hover:bg-surface disabled:opacity-20 text-hi z-10 text-lg"
            >
              <IconChevronLeft size={18} />
            </button>

            {currentPage
              ? (
                <div className="rounded-xl overflow-hidden shadow-panel">
                  <PagePreview page={currentPage} paper={zine.paperSize} caption={currentCap} thumbs={thumbnails} />
                </div>
              )
              : <div className="text-lo text-sm flex items-center gap-1">No pages. Add one below <IconChevronRight size={14} /></div>
            }

            <button
              onClick={() => setSelectedIdx(i => Math.min(pages.length - 1, i + 1))}
              disabled={selectedIdx >= pages.length - 1}
              className="absolute right-4 top-1/2 -translate-y-1/2 p-2 rounded-full
                bg-surface/80 hover:bg-surface disabled:opacity-20 text-hi z-10 text-lg"
            >
              <IconChevronRight size={18} />
            </button>

            {pages.length > 0 && (
              <div className="absolute bottom-4 left-1/2 -translate-x-1/2 text-xs text-lo bg-surface/80 rounded-full px-3 py-1">
                {selectedIdx + 1} / {pages.length}
              </div>
            )}
          </div>

          {/* Page strip */}
          <div className="shrink-0 vibrancy-panel border-t border-border px-4 py-4 overflow-x-auto">
            <DndContext sensors={sensors} collisionDetection={closestCenter} onDragEnd={onDragEnd}>
              <SortableContext items={pages.map(p => p.id)} strategy={horizontalListSortingStrategy}>
                <div className="flex gap-3 items-start">
                  {pages.map((page, i) => (
                    <SortablePage
                      key={page.id}
                      page={page}
                      isSelected={i === selectedIdx}
                      thumb={page.imageId ? thumbnails[page.imageId] : undefined}
                      onSelect={() => setSelectedIdx(i)}
                      onRemove={() => onRemovePage(page.id)}
                    />
                  ))}

                  {/* Add image page */}
                  <button
                    onClick={() => setPickerMode({ mode: 'add' })}
                    className="w-24 h-32 rounded-xl border-2 border-dashed border-border
                      hover:border-amber-500/60 hover:bg-surface/50 flex flex-col items-center
                      justify-center gap-1.5 text-lo hover:text-amber-400 transition-all shrink-0"
                    title="Add a page from session images"
                  >
                    <IconPlus size={18} />
                    <span className="text-[10px]">Add image</span>
                  </button>

                  {/* Add blank/title page */}
                  <button
                    onClick={onAddBlankPage}
                    className="w-24 h-32 rounded-xl border-2 border-dashed border-border
                      hover:border-mid hover:bg-surface/50 flex flex-col items-center
                      justify-center gap-1.5 text-lo hover:text-mid transition-all shrink-0"
                    title="Add a blank title page"
                  >
                    <IconPlus size={18} />
                    <span className="text-[10px]">Title page</span>
                  </button>
                </div>
              </SortableContext>
            </DndContext>
          </div>
        </div>

        <Inspector
          currentPage={currentPage}
          pagesEmpty={pages.length === 0}
          currentCap={currentCap}
          currentThumb={currentThumb}
          suggestions={suggestions}
          setPickerMode={setPickerMode}
          onClearPageImage={onClearPageImage}
          onTemplate={onTemplate}
          editingOvr={editingOvr}
          setEditingOvr={setEditingOvr}
          overrideText={overrideText}
          setOverrideText={setOverrideText}
          saveOverride={saveOverride}
          clearOverride={clearOverride}
        />
      </div>

      {/* ── Image picker modal ───────────────────────────────────────────────── */}
      {pickerMode && (
        <ImagePicker
          images={images}
          thumbnails={thumbnails}
          usedImageIds={usedImageIds}
          onPick={onPickImage}
          onClose={() => setPickerMode(null)}
        />
      )}
    </div>
  )
}
