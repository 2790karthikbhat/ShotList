// Typed wrappers around Tauri invoke() calls.
// All backend commands are centralised here — nothing in components calls
// invoke() directly.

import { invoke } from '@tauri-apps/api/core'
import { convertFileSrc } from '@tauri-apps/api/core'
import type { SessionInfo, ImageInfo, FilterState, AppSettings, ImageAnalysis, DevelopParams, Caption, Zine, ZinePage, TemplateId, PaperSize, DirEntry, RootLocation } from '../types'

export { convertFileSrc }

// ── Session ───────────────────────────────────────────────────────────────────

export const importFolder = () =>
  invoke<SessionInfo>('import_folder')

export const openFolderAt = (path: string) =>
  invoke<SessionInfo>('open_folder_at', { path })

export const getSession = () =>
  invoke<SessionInfo | null>('get_session')

// ── Folder sidebar ────────────────────────────────────────────────────────────

export const listDirEntries = (path: string) =>
  invoke<DirEntry[]>('list_dir_entries', { path })

export const getRootLocations = () =>
  invoke<RootLocation[]>('get_root_locations')

export const previewFile = (path: string) =>
  invoke<string>('preview_file', { path })

export const removeRecentFolder = (path: string) =>
  invoke<void>('remove_recent_folder', { path })

// ── Images ────────────────────────────────────────────────────────────────────

export const getVisibleImages = () =>
  invoke<ImageInfo[]>('get_visible_images')

export const getCurrentImage = () =>
  invoke<ImageInfo | null>('get_current_image')

// ── Navigation ────────────────────────────────────────────────────────────────

export const navigate = (direction: 'next' | 'prev') =>
  invoke<string | null>('navigate', { direction })

export const jumpToImage = (index: number) =>
  invoke<void>('jump_to_image', { index })

// ── Viewer ────────────────────────────────────────────────────────────────────

export const getImageSrc = (imageId: string, pristine?: boolean) =>
  invoke<string>('get_image_src', { imageId, pristine })

// ── Cull decisions ────────────────────────────────────────────────────────────

export const setCullState = (path: string, cullState: string) =>
  invoke<SessionInfo>('set_cull_state', { path, cullState })

export const setRating = (path: string, rating: number) =>
  invoke<void>('set_rating', { path, rating })

// ── Thumbnails ────────────────────────────────────────────────────────────────

export const requestThumbnails = (imageIds: string[]) =>
  invoke<void>('request_thumbnails', { imageIds })

export const getZineThumbUrls = (imageIds: string[]) =>
  invoke<Record<string, string>>('get_zine_thumb_urls', { imageIds })

// ── Technical analysis ────────────────────────────────────────────────────────

export const getImageAnalysis = (imageId: string) =>
  invoke<ImageAnalysis>('get_image_analysis', { imageId })

// ── Bulk operations ───────────────────────────────────────────────────────────

export const exportPicks = () =>
  invoke<string>('export_picks')

export const runAutoCull = () =>
  invoke<string>('run_auto_cull')

// ── Filter ────────────────────────────────────────────────────────────────────

export const setFilter = (filter: FilterState) =>
  invoke<ImageInfo[]>('set_filter', {
    filter: {
      showUnreviewed: filter.showUnreviewed,
      showPicks:      filter.showPicks,
      showRejects:    filter.showRejects,
      showFlagged:    filter.showFlagged,
      minRating:      filter.minRating,
    },
  })

// ── Caption stage ─────────────────────────────────────────────────────────────

export const getCaption = (imageId: string) =>
  invoke<Caption>('get_caption', { imageId })

export const saveCaption = (imageId: string, caption: Caption) =>
  invoke<void>('save_caption', { imageId, caption })

export const generateCaption = (imageId: string) =>
  invoke<Caption>('generate_caption', { imageId })

export const embedCaptions = () =>
  invoke<string>('embed_captions')

// ── Develop export ────────────────────────────────────────────────────────────

export const exportDeveloped = (imageId: string, params: DevelopParams, quality: number) =>
  invoke<string>('export_developed', { imageId, params, quality })

export const exportAllPicks = (quality: number) =>
  invoke<string>('export_all_picks', { quality })

// ── Develop stage ─────────────────────────────────────────────────────────────

export const getDevelopParams = (imageId: string) =>
  invoke<DevelopParams>('get_develop_params', { imageId })

export const saveDevelopParams = (imageId: string, params: DevelopParams) =>
  invoke<void>('save_develop_params', { imageId, params })

export const getDevelopPreview = (imageId: string, params: DevelopParams) =>
  invoke<string>('get_develop_preview', { imageId, params })

export const autoDevelop = (imageId: string) =>
  invoke<void>('auto_develop', { imageId })

// ── Zine builder ─────────────────────────────────────────────────────────────

export const createOrGetZine = () =>
  invoke<Zine>('create_or_get_zine')

export const addZinePage = (zineId: string, imageId: string | null, template: TemplateId) =>
  invoke<ZinePage>('add_zine_page', { zineId, imageId, template })

export const removeZinePage = (pageId: string) =>
  invoke<void>('remove_zine_page', { pageId })

export const generateAllCaptions = (imageIds: string[], overwrite = false) =>
  invoke<string>('generate_all_captions', { imageIds, overwrite })

export const setPageImage = (pageId: string, imageId: string | null, slot = 1) =>
  invoke<void>('set_page_image', { pageId, imageId, slot })

export const getZine = () =>
  invoke<Zine>('get_zine')

export const updatePageOrder = (pageIds: string[]) =>
  invoke<void>('update_page_order', { pageIds })

export const setPageTemplate = (pageId: string, template: TemplateId) =>
  invoke<void>('set_page_template', { pageId, template })

export const setPageCaptionOverride = (pageId: string, override_: string | null) =>
  invoke<void>('set_page_caption_override', { pageId, override_ })

export const setZinePaperSize = (zineId: string, paperSize: PaperSize) =>
  invoke<void>('set_zine_paper_size', { zineId, paperSize })

export const exportPdf = (quality: number) =>
  invoke<string>('export_pdf', { quality })

export const aiSequence = () =>
  invoke<string>('ai_sequence')

// ── Settings ──────────────────────────────────────────────────────────────────

export const getSettings = () =>
  invoke<AppSettings>('get_settings')

export const saveSettings = (settings: AppSettings) =>
  invoke<void>('save_settings', { settings })

export const testLmStudioConnection = (baseUrl: string) =>
  invoke<string[]>('test_lmstudio_connection', { baseUrl })

export const openWithApp = (imagePath: string, appName?: string) =>
  invoke<void>('open_with_app', { imagePath, appName: appName ?? null })
