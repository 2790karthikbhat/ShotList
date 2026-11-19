// Global application state managed by Zustand.
// The store is the single source of truth for the React UI — all Tauri IPC
// calls update the store, and components read from it.

import { create } from 'zustand'
import { listen } from '@tauri-apps/api/event'

import * as ipc from './lib/ipc'
import type {
  SessionInfo,
  ImageInfo,
  FilterState,
  Stage,
  Toast,
  ToastKind,
} from './types'
import { FILTER_ALL } from './types'

interface ThumbnailEvent {
  id: string
  url: string       // data:image/jpeg;base64,... — no asset protocol needed
  highRes: boolean
  focusScore: number
  faceCount: number
}

interface AppStore {
  // ── Session ───────────────────────────────────────────────────────────
  session:     SessionInfo | null
  images:      Record<string, ImageInfo>   // keyed by id
  visibleIds:  string[]                    // ordered for filmstrip
  currentId:   string | null

  // ── Display ───────────────────────────────────────────────────────────
  thumbnails:  Record<string, string>      // id → object URL
  viewerSrc:   string                      // URL for the main viewer image
  viewerLoading: boolean

  // ── UI state ──────────────────────────────────────────────────────────
  stage:       Stage
  filter:      FilterState
  toast:       Toast | null
  toastTimer:  ReturnType<typeof setTimeout> | null
  settingsOpen: boolean

  // ── Actions ───────────────────────────────────────────────────────────
  importFolder:  () => Promise<void>
  openFolderAt:  (path: string) => Promise<void>
  openFolderAtImage: (folderPath: string, targetPath: string) => Promise<void>
  navigate:      (dir: 'next' | 'prev') => Promise<void>
  jumpTo:        (index: number) => Promise<void>
  setCullState:  (path: string, state: string) => Promise<void>
  setRating:     (path: string, rating: number) => Promise<void>
  exportPicks:   () => Promise<void>
  autoCull:      () => Promise<void>
  applyFilter:   (f: FilterState) => Promise<void>
  setStage:      (s: Stage) => void
  showToast:     (msg: string, kind: ToastKind) => void
  dismissToast:  () => void
  openSettings:  () => void
  closeSettings: () => void
  initListeners: () => Promise<() => void>   // returns cleanup fn
  loadViewerFor: (id: string) => Promise<void>
}

let toastSeq = 0

export const useStore = create<AppStore>((set, get) => {
  // Shared by importFolder (native picker), openFolderAt (sidebar tree), and
  // openFolderAtImage (sidebar Quick Look) — all land here once a SessionInfo
  // has been loaded from the backend.
  const applySession = async (
    session: SessionInfo,
    opts?: { stage?: Stage; focusPath?: string },
  ) => {
    const images = await ipc.getVisibleImages()
    const byId = Object.fromEntries(images.map(img => [img.id, img]))
    const ids  = images.map(i => i.id)

    // If opened via Quick Look's "Open in Develop", land on that exact photo
    // instead of the session's default first image.
    const focused = opts?.focusPath
      ? images.find(img => img.path === opts.focusPath)
      : undefined
    const currentId = focused?.id ?? session.currentImageId

    set({
      session,
      images:     byId,
      visibleIds: ids,
      currentId,
      thumbnails: {},
      viewerSrc:  '',
      stage:      opts?.stage ?? 'cull',
    })

    // Kick off thumbnail decodes for the first ~30 visible images.
    const firstBatch = ids.slice(0, 30)
    ipc.requestThumbnails(firstBatch).catch(() => {})

    // Load the viewer for the current image.
    if (currentId) {
      get().loadViewerFor(currentId)
    }

    get().showToast(
      `Imported ${session.total} image(s) from ${session.name}`,
      'success',
    )
  }

  return {
  session:      null,
  images:       {},
  visibleIds:   [],
  currentId:    null,
  thumbnails:   {},
  viewerSrc:    '',
  viewerLoading: false,
  stage:        'cull',
  filter:       FILTER_ALL,
  toast:        null,
  toastTimer:   null,
  settingsOpen: false,

  // ── Import ─────────────────────────────────────────────────────────────
  importFolder: async () => {
    try {
      const session = await ipc.importFolder()
      await applySession(session)
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : String(e)
      if (msg !== 'cancelled') get().showToast(`Import failed: ${msg}`, 'error')
    }
  },

  openFolderAt: async (path) => {
    try {
      const session = await ipc.openFolderAt(path)
      await applySession(session)
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : String(e)
      get().showToast(`Couldn't open folder: ${msg}`, 'error')
    }
  },

  openFolderAtImage: async (folderPath, targetPath) => {
    try {
      const session = await ipc.openFolderAt(folderPath)
      await applySession(session, { stage: 'develop', focusPath: targetPath })
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : String(e)
      get().showToast(`Couldn't open folder: ${msg}`, 'error')
    }
  },

  // ── Navigation ──────────────────────────────────────────────────────────
  navigate: async (dir) => {
    try {
      const newId = await ipc.navigate(dir)
      if (!newId) return
      set({ currentId: newId })
      get().loadViewerFor(newId)

      // Lazy-load thumbnails around the new position.
      const { visibleIds } = get()
      const pos = visibleIds.indexOf(newId)
      if (pos >= 0) {
        const batch = visibleIds.slice(
          Math.max(0, pos - 5),
          Math.min(visibleIds.length, pos + 20),
        )
        ipc.requestThumbnails(batch.filter(id => !get().thumbnails[id])).catch(() => {})
      }
    } catch (e) {
      console.error('navigate error', e)
    }
  },

  jumpTo: async (index) => {
    try {
      await ipc.jumpToImage(index)
      const img = await ipc.getCurrentImage()
      if (!img) return
      set({ currentId: img.id })
      get().loadViewerFor(img.id)
    } catch (e) {
      console.error('jumpTo error', e)
    }
  },

  // ── Viewer ────────────────────────────────────────────────────────────
  loadViewerFor: async (id: string) => {
    // Clear immediately — never show a stale image for a new currentId.
    set({ viewerSrc: '', viewerLoading: true })
    try {
      // get_image_src returns a data: URL (JPEG/PNG) or '' (RAW still decoding).
      const dataUrl = await ipc.getImageSrc(id)
      // Guard: user may have navigated away while the IPC call was in flight.
      if (get().currentId !== id) return
      if (dataUrl) {
        set({ viewerSrc: dataUrl, viewerLoading: false })
      } else {
        // RAW not cached yet — thumbnail-ready(highRes=true) event will deliver the URL.
        set({ viewerSrc: '', viewerLoading: true })
      }
    } catch {
      set({ viewerLoading: false })
    }
  },

  // ── Cull decisions ────────────────────────────────────────────────────
  setCullState: async (path, state) => {
    try {
      const session = await ipc.setCullState(path, state)
      const img = Object.values(get().images).find(i => i.path === path)
      if (img) {
        const updatedImg: ImageInfo = { ...img, cullState: state as ImageInfo['cullState'] }
        const updatedImages = { ...get().images, [img.id]: updatedImg }
        // Re-derive visibleIds so the filmstrip drops images filtered out after culling.
        const { filter } = get()
        const newVisibleIds = get().visibleIds.filter(id => {
          const i = updatedImages[id]
          if (!i) return false
          if (i.rating < filter.minRating) return false
          if (i.cullState === 'pick') return filter.showPicks
          if (i.cullState === 'reject') return filter.showRejects
          if (i.cullState === 'flagged') return filter.showFlagged
          return filter.showUnreviewed
        })
        set({ session, currentId: session.currentImageId, images: updatedImages, visibleIds: newVisibleIds })
        if (session.currentImageId && session.currentImageId !== img.id) {
          get().loadViewerFor(session.currentImageId)
        }
      }
    } catch (e) {
      get().showToast(`Error: ${e}`, 'error')
    }
  },

  setRating: async (path, rating) => {
    try {
      await ipc.setRating(path, rating)
      const img = Object.values(get().images).find(i => i.path === path)
      if (img) {
        set(s => ({ images: { ...s.images, [img.id]: { ...img, rating } } }))
      }
    } catch (e) {
      get().showToast(`Error: ${e}`, 'error')
    }
  },

  // ── Bulk ops ──────────────────────────────────────────────────────────
  exportPicks: async () => {
    try {
      const msg = await ipc.exportPicks()
      get().showToast(msg, 'success')
    } catch (e) {
      get().showToast(`Export failed: ${e}`, 'error')
    }
  },

  autoCull: async () => {
    try {
      const msg = await ipc.runAutoCull()
      // Re-fetch visible images since states changed.
      const images = await ipc.getVisibleImages()
      const byId = Object.fromEntries(images.map(img => [img.id, img]))
      set({ images: byId })
      get().showToast(msg, 'success')
    } catch (e) {
      get().showToast(`Auto-cull: ${e}`, 'warning')
    }
  },

  // ── Filter ────────────────────────────────────────────────────────────
  applyFilter: async (f) => {
    try {
      const images = await ipc.setFilter(f)
      const byId = Object.fromEntries(images.map(img => [img.id, img]))
      const ids  = images.map(i => i.id)

      // The previously-selected image may have been filtered out by the new
      // filter (e.g. switching from Picks to Rejects) — `images` is replaced
      // wholesale above, so a stale currentId would point at an id that no
      // longer exists, leaving the viewer blank. Fall back to the new
      // visible set's first image (or clear entirely if it's empty).
      const prevId = get().currentId
      const stillVisible = !!prevId && !!byId[prevId]

      set({ filter: f, images: byId, visibleIds: ids })

      if (!stillVisible) {
        const nextId = ids[0] ?? null
        if (nextId) {
          set({ currentId: nextId })
          get().loadViewerFor(nextId)
        } else {
          set({ currentId: null, viewerSrc: '' })
        }
      }
    } catch (e) {
      console.error('filter error', e)
    }
  },

  // ── Stage ─────────────────────────────────────────────────────────────
  setStage: (stage) => set({ stage }),

  // ── Toast ─────────────────────────────────────────────────────────────
  showToast: (message, kind) => {
    const prev = get().toastTimer
    if (prev) clearTimeout(prev)
    const timer = setTimeout(() => set({ toast: null, toastTimer: null }), 3500)
    set({ toast: { message, kind, id: ++toastSeq }, toastTimer: timer })
  },

  dismissToast: () => {
    const prev = get().toastTimer
    if (prev) clearTimeout(prev)
    set({ toast: null, toastTimer: null })
  },

  // ── Settings ─────────────────────────────────────────────────────────
  openSettings:  () => set({ settingsOpen: true }),
  closeSettings: () => set({ settingsOpen: false }),

  // ── Event listeners ───────────────────────────────────────────────────
  initListeners: async () => {
    const unlisten = await listen<ThumbnailEvent>('thumbnail-ready', (event) => {
      const { id, url, highRes } = event.payload
      if (highRes) {
        if (get().currentId === id) {
          set({ viewerSrc: url, viewerLoading: false })
        }
      } else {
        set(s => ({ thumbnails: { ...s.thumbnails, [id]: url } }))
        const img = get().images[id]
        if (img && event.payload.focusScore !== undefined) {
          set(s => ({
            images: {
              ...s.images,
              [id]: { ...img, focusScore: event.payload.focusScore },
            },
          }))
        }
      }
    })
    return unlisten
  },
  }
})
