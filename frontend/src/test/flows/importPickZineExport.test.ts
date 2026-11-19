// Smoke test for the Import -> Pick -> Zine -> Export path end to end.
//
// Import and Pick are exercised through the real Zustand store — the exact
// logic CullStage.tsx triggers. Zine/Export are exercised directly against
// the `lib/ipc` wrappers in the same sequence ZineStage.tsx calls them:
// full RTL rendering of that component is tracked separately as a stretch
// item (see docs/plans/release-hardening/00-overview.md — tauri-driver E2E)
// rather than attempted here.
import { describe, it, expect, vi, beforeEach } from 'vitest'

const mockInvokeImpl = vi.fn()

vi.mock('@tauri-apps/api/core', () => ({
  invoke: (cmd: string, args?: unknown) => mockInvokeImpl(cmd, args),
  convertFileSrc: (p: string) => p,
}))

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}))

import { useStore } from '../../store'
import * as ipc from '../../lib/ipc'
import { FILTER_ALL } from '../../types'
import type { SessionInfo, ImageInfo, Zine, ZinePage } from '../../types'

function makeImage(overrides: Partial<ImageInfo> = {}): ImageInfo {
  return {
    id: 'img-1',
    path: '/photos/img-1.jpg',
    filename: 'img-1.jpg',
    cullState: 'unreviewed',
    rating: 0,
    focusScore: null,
    faceCount: 0,
    groupId: null,
    groupSize: null,
    iso: null,
    shutterSpeed: null,
    aperture: null,
    focalLengthMm: null,
    lensModel: null,
    sizeLabel: '',
    exposureBlown: false,
    exposureCrushed: false,
    isRaw: false,
    fullImageIndex: 0,
    ...overrides,
  }
}

function makeSession(overrides: Partial<SessionInfo> = {}): SessionInfo {
  return {
    id: 'sess-1',
    name: 'Test Folder',
    total: 1,
    picks: 0,
    rejects: 0,
    flagged: 0,
    unreviewed: 1,
    cursor: 0,
    currentImageId: 'img-1',
    progressPct: 100,
    ...overrides,
  }
}

beforeEach(() => {
  mockInvokeImpl.mockReset()
  useStore.setState({
    session: null,
    images: {},
    visibleIds: [],
    currentId: null,
    thumbnails: {},
    viewerSrc: '',
    viewerLoading: false,
    filter: FILTER_ALL,
    toast: null,
    toastTimer: null,
  })
})

describe('import -> pick -> zine -> export', () => {
  it('imports a folder, picks an image, builds a zine page, and exports the PDF', async () => {
    const image = makeImage()
    const session = makeSession()
    // currentImageId stays 'img-1' after picking — the still-current image —
    // so setCullState won't trigger a second get_image_src call.
    const pickedSession = makeSession({ picks: 1, unreviewed: 0 })

    const zine: Zine = { id: 'zine-1', sessionId: session.id, paperSize: 'a5', pages: [] }
    const page: ZinePage = {
      id: 'page-1',
      zineId: zine.id,
      position: 0,
      imageId: null,
      imageId2: null,
      imageId3: null,
      imageId4: null,
      imageId5: null,
      imageId6: null,
      imageId7: null,
      imageId8: null,
      imageId9: null,
      template: 'fullBleed',
      captionOverride: null,
    }

    mockInvokeImpl.mockImplementation((cmd: string) => {
      switch (cmd) {
        case 'import_folder':
          return Promise.resolve(session)
        case 'get_visible_images':
          return Promise.resolve([image])
        case 'get_image_src':
          return Promise.resolve('data:image/jpeg;base64,xyz')
        case 'set_cull_state':
          return Promise.resolve(pickedSession)
        case 'create_or_get_zine':
          return Promise.resolve(zine)
        case 'add_zine_page':
          return Promise.resolve(page)
        case 'set_page_image':
          return Promise.resolve()
        case 'export_pdf':
          return Promise.resolve('/photos/Zine/zine.pdf')
        default:
          return Promise.resolve(undefined)
      }
    })

    // 1. Import a folder.
    await useStore.getState().importFolder()
    expect(useStore.getState().session?.id).toBe('sess-1')
    expect(useStore.getState().images['img-1']).toBeDefined()

    // 2. Pick the image.
    await useStore.getState().setCullState(image.path, 'pick')
    expect(useStore.getState().images['img-1'].cullState).toBe('pick')

    // 3. Build a zine page and assign the picked image to it.
    const createdZine = await ipc.createOrGetZine()
    const createdPage = await ipc.addZinePage(createdZine.id, null, 'fullBleed')
    await ipc.setPageImage(createdPage.id, image.id, 1)

    // 4. Export the PDF.
    const outPath = await ipc.exportPdf(90)
    expect(outPath).toBe('/photos/Zine/zine.pdf')

    // The whole flow hit the backend in exactly this order.
    expect(mockInvokeImpl.mock.calls.map(([cmd]) => cmd)).toEqual([
      'import_folder',
      'get_visible_images',
      'request_thumbnails',
      'get_image_src',
      'set_cull_state',
      'create_or_get_zine',
      'add_zine_page',
      'set_page_image',
      'export_pdf',
    ])
    expect(mockInvokeImpl).toHaveBeenCalledWith('set_page_image', {
      pageId: 'page-1',
      imageId: 'img-1',
      slot: 1,
    })
    expect(mockInvokeImpl).toHaveBeenCalledWith('export_pdf', { quality: 90 })
  })
})
