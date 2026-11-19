// Mirrors the Rust DTOs from src/commands.rs

export type CullState = 'pick' | 'reject' | 'flagged' | 'unreviewed'
export type Stage = 'cull' | 'analyse' | 'develop' | 'caption' | 'zine'
export type ToastKind = 'info' | 'success' | 'warning' | 'error'

export interface SessionInfo {
  id: string
  name: string
  total: number
  picks: number
  rejects: number
  flagged: number
  unreviewed: number
  cursor: number
  currentImageId: string | null
  progressPct: number
}

export interface ImageInfo {
  id: string
  path: string
  filename: string
  cullState: CullState
  rating: number
  focusScore: number | null
  faceCount: number
  groupId: number | null
  groupSize: number | null
  iso: number | null
  shutterSpeed: string | null
  aperture: number | null
  focalLengthMm: number | null
  lensModel: string | null
  sizeLabel: string
  exposureBlown: boolean
  exposureCrushed: boolean
  isRaw: boolean
  fullImageIndex: number
}

export interface FilterState {
  showUnreviewed: boolean
  showPicks: boolean
  showRejects: boolean
  showFlagged: boolean
  minRating: number
}

export interface Toast {
  message: string
  kind: ToastKind
  id: number
}

export interface AppSettings {
  anthropicKey: string
  openaiKey: string
  autoAdvance: boolean
  showRejects: boolean
  aiProvider: string      // 'anthropic' | 'lmstudio'
  lmstudioUrl: string    // e.g. 'http://localhost:1234/v1'
  lmstudioModel: string  // model ID loaded in LM Studio
  recentFolders: string[]
  theme: 'dark' | 'light' | 'system'
}

// ── Folder sidebar ───────────────────────────────────────────────────────────

export interface DirEntry {
  name: string
  path: string
  kind: 'folder' | 'image'
}

export interface RootLocation {
  label: string
  path: string
}

// ── Technical analysis ────────────────────────────────────────────────────────

export interface ImageAnalysis {
  /** Per-channel histograms, 256 buckets each, normalised 0–1 to global peak. */
  histR:   number[]
  histG:   number[]
  histB:   number[]
  histLum: number[]
  /** Fraction of pixels in each Ansel Adams zone (Zone 0 = index 0 … Zone X = index 10). */
  zonePcts: number[]
  /** Flat (row-major) normalised Laplacian energy per grid cell, 0 = soft, 1 = sharpest. */
  focusMap:      number[]
  focusMapCols:  number
  focusMapRows:  number
  blownPct:      number
  crushedPct:    number
  meanLuma:      number
  colorTempK:    number | null
}

// ── Caption stage ─────────────────────────────────────────────────────────────

export interface Caption {
  title:       string
  description: string
  keywords:    string[]
  location:    string
  credit:      string
}

export const DEFAULT_CAPTION: Caption = {
  title: '', description: '', keywords: [], location: '', credit: '',
}

/** One caption field arriving from the Claude stream. */
export interface CaptionFieldEvent {
  field: string
  value: string | string[]
}

// ── Develop export ────────────────────────────────────────────────────────────

export interface ExportProgress {
  done:     number
  total:    number
  filename: string
}

// ── Develop stage ─────────────────────────────────────────────────────────────

export interface CropRect {
  x:      number  // normalized 0..1, relative to the rotated frame
  y:      number
  width:  number
  height: number
}

export interface DevelopParams {
  // Geometry
  rotation: number      // clockwise degrees: 0 | 90 | 180 | 270
  crop:     CropRect | null
  // Basic
  exposure:   number   // EV, -5..+5
  contrast:   number   // -100..+100
  highlights: number   // -100..+100
  shadows:    number   // -100..+100
  whites:     number   // -100..+100
  blacks:     number   // -100..+100
  // Presence
  clarity:    number   // -100..+100
  vibrance:   number   // -100..+100
  saturation: number   // -100..+100
  // White balance
  temperature: number  // -100..+100
  tint:        number  // -100..+100
  // Effects
  vignette: number     // -100..+100
  grain:    number     // 0..+100
  // B&W
  bwEnabled: boolean
  bwRed:    number
  bwOrange: number
  bwYellow: number
  bwGreen:  number
  bwAqua:   number
  bwBlue:   number
}

export const DEFAULT_DEVELOP_PARAMS: DevelopParams = {
  rotation: 0, crop: null,
  exposure: 0, contrast: 0, highlights: 0, shadows: 0, whites: 0, blacks: 0,
  clarity: 0, vibrance: 0, saturation: 0, temperature: 0, tint: 0,
  vignette: 0, grain: 0,
  bwEnabled: false,
  bwRed: 0, bwOrange: 0, bwYellow: 0, bwGreen: 0, bwAqua: 0, bwBlue: 0,
}

/** A single Claude auto-develop suggestion streamed in real-time. */
export interface DevelopSuggestion {
  param:  string
  value:  number
  reason: string
}

// ── Zine builder ──────────────────────────────────────────────────────────────

export type PaperSize  = 'a4' | 'a5' | 'square' | 'half-letter'
export type TemplateId =
  | 'fullBleed' | 'top75' | 'left50' | 'right50' | 'gridTwo' | 'titlePage'
  | 'pullQuote' | 'imageLead' | 'panoramic'
  | 'editorial' | 'triptych' | 'grid4x' | 'featureSpread'
  | 'photoEssay' | 'contactSheet'

export interface ZinePage {
  id:              string
  zineId:          string
  position:        number
  imageId:         string | null
  imageId2:        string | null
  imageId3:        string | null
  imageId4:        string | null
  imageId5:        string | null
  imageId6:        string | null
  imageId7:        string | null
  imageId8:        string | null
  imageId9:        string | null
  template:        TemplateId
  captionOverride: string | null
}

export interface Zine {
  id:        string
  sessionId: string
  paperSize: PaperSize
  pages:     ZinePage[]
}

export interface SequenceSuggestion {
  position: number
  imageId:  string
  reason:   string
}

export interface PdfExportProgress {
  page:    number
  total:   number
  message: string
}

export const PAPER_SIZE_LABELS: Record<PaperSize, string> = {
  'a4':          'A4 (210 × 297 mm)',
  'a5':          'A5 (148 × 210 mm)',
  'square':      'Square (200 × 200 mm)',
  'half-letter': 'Half Letter (140 × 216 mm)',
}

export const TEMPLATE_LABELS: Record<TemplateId, string> = {
  fullBleed:     'Full Bleed',
  top75:         'Top 75%',
  left50:        'Left 50%',
  right50:       'Right 50%',
  gridTwo:       'Grid 2×1',
  titlePage:     'Title Page',
  pullQuote:     'Pull Quote',
  imageLead:     'Image Lead',
  panoramic:     'Panoramic',
  editorial:     'Editorial',
  triptych:      'Triptych',
  grid4x:        'Grid 4×',
  featureSpread: 'Feature Spread',
  photoEssay:    'Photo Essay',
  contactSheet:  'Contact Sheet',
}

export const FILTER_ALL: FilterState = {
  showUnreviewed: true,
  showPicks: true,
  showRejects: true,
  showFlagged: true,
  minRating: 0,
}

export const CULL_STATE_COLORS: Record<CullState, string> = {
  pick:       '#50C378',
  reject:     '#D24646',
  flagged:    '#DCA032',
  unreviewed: 'transparent',
}
