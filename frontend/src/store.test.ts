// Unit tests for the Zustand store's actual business logic: session load,
// cancellation handling, and the cull-state/filter re-derivation that keeps
// the filmstrip in sync without a full backend round-trip.
import { describe, it, expect, vi, beforeEach } from 'vitest'

const mockInvokeImpl = vi.fn()

vi.mock('@tauri-apps/api/core', () => ({
  invoke: (cmd: string, args?: unknown) => mockInvokeImpl(cmd, args),
  convertFileSrc: (p: string) => p,
}))

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}))

import { useStore } from './store'
import { FILTER_ALL } from './types'
import type { SessionInfo, ImageInfo } from './types'

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

describe('importFolder', () => {
  it('loads the session, images, and focuses the current image', async () => {
    const session = makeSession()
    const image = makeImage()
    mockInvokeImpl.mockImplementation((cmd: string) => {
      switch (cmd) {
        case 'import_folder':
          return Promise.resolve(session)
        case 'get_visible_images':
          return Promise.resolve([image])
        case 'get_image_src':
          return Promise.resolve('data:image/jpeg;base64,xyz')
        default:
          return Promise.resolve(undefined)
      }
    })

    await useStore.getState().importFolder()

    const state = useStore.getState()
    expect(state.session).toEqual(session)
    expect(state.images['img-1']).toEqual(image)
    expect(state.visibleIds).toEqual(['img-1'])
    expect(state.currentId).toBe('img-1')
    expect(state.stage).toBe('cull')
    expect(state.toast?.kind).toBe('success')
  })

  it('does not show an error toast when the folder picker is cancelled', async () => {
    mockInvokeImpl.mockImplementation((cmd: string) => {
      if (cmd === 'import_folder') return Promise.reject(new Error('cancelled'))
      return Promise.resolve(undefined)
    })

    await useStore.getState().importFolder()

    expect(useStore.getState().toast).toBeNull()
  })

  it('shows an error toast on a real import failure', async () => {
    mockInvokeImpl.mockImplementation((cmd: string) => {
      if (cmd === 'import_folder') return Promise.reject(new Error('disk read error'))
      return Promise.resolve(undefined)
    })

    await useStore.getState().importFolder()

    const toast = useStore.getState().toast
    expect(toast?.kind).toBe('error')
    expect(toast?.message).toContain('disk read error')
  })
})

describe('setCullState', () => {
  it('marks an image picked and drops it from a Rejects-only filter', async () => {
    const image = makeImage({ cullState: 'unreviewed' })
    useStore.setState({
      images: { 'img-1': image },
      visibleIds: ['img-1'],
      filter: {
        ...FILTER_ALL,
        showUnreviewed: false,
        showPicks: false,
        showRejects: true,
        showFlagged: false,
      },
    })

    const updatedSession = makeSession({ picks: 1, unreviewed: 0, currentImageId: null })
    mockInvokeImpl.mockImplementation((cmd: string) => {
      if (cmd === 'set_cull_state') return Promise.resolve(updatedSession)
      return Promise.resolve(undefined)
    })

    await useStore.getState().setCullState(image.path, 'pick')

    const state = useStore.getState()
    expect(state.images['img-1'].cullState).toBe('pick')
    // Filter only shows rejects, so the newly-picked image drops out of the filmstrip.
    expect(state.visibleIds).toEqual([])
    expect(mockInvokeImpl).toHaveBeenCalledWith('set_cull_state', {
      path: image.path,
      cullState: 'pick',
    })
  })

  it('keeps a picked image visible when the filter allows picks', async () => {
    const image = makeImage({ cullState: 'unreviewed' })
    useStore.setState({
      images: { 'img-1': image },
      visibleIds: ['img-1'],
      filter: FILTER_ALL,
    })

    const updatedSession = makeSession({ picks: 1, unreviewed: 0, currentImageId: null })
    mockInvokeImpl.mockImplementation((cmd: string) => {
      if (cmd === 'set_cull_state') return Promise.resolve(updatedSession)
      return Promise.resolve(undefined)
    })

    await useStore.getState().setCullState(image.path, 'pick')

    expect(useStore.getState().visibleIds).toEqual(['img-1'])
  })
})

describe('applyFilter', () => {
  it('falls back to the first still-visible image when the current one is filtered out', async () => {
    useStore.setState({ currentId: 'img-1' })
    const image2 = makeImage({ id: 'img-2', path: '/photos/img-2.jpg' })
    mockInvokeImpl.mockImplementation((cmd: string) => {
      if (cmd === 'set_filter') return Promise.resolve([image2])
      if (cmd === 'get_image_src') return Promise.resolve('data:image/jpeg;base64,xyz')
      return Promise.resolve(undefined)
    })

    await useStore.getState().applyFilter(FILTER_ALL)

    const state = useStore.getState()
    expect(state.visibleIds).toEqual(['img-2'])
    expect(state.currentId).toBe('img-2')
  })

  it('clears the current image when the new filter has no matches', async () => {
    useStore.setState({ currentId: 'img-1' })
    mockInvokeImpl.mockImplementation((cmd: string) => {
      if (cmd === 'set_filter') return Promise.resolve([])
      return Promise.resolve(undefined)
    })

    await useStore.getState().applyFilter(FILTER_ALL)

    const state = useStore.getState()
    expect(state.visibleIds).toEqual([])
    expect(state.currentId).toBeNull()
    expect(state.viewerSrc).toBe('')
  })
})
