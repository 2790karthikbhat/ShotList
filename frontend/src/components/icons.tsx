// Thin re-exports of lucide-react (already a dependency — see ZineStage.tsx)
// under the names the rest of the app already imports. Keeps every call site
// unchanged while swapping the hand-rolled SVG set (inconsistent sizing,
// IconSparkle in particular rendered visibly smaller than its siblings) for
// a properly-drawn icon library.
//
// Sizing convention: every icon defaults to 1em, so size follows the
// surrounding font-size class. Chrome-context icons (toolbar, sidebar, nav
// rows) should sit in a text-xl/text-2xl (20-24px) context; dense/inline
// icons (chips, stat badges, inline metadata) stay text-xs/text-sm.
// stages/zine/* imports icons straight from lucide-react rather than through
// here — route new zine icon usage through this file too, for one convention.

import type { LucideIcon, LucideProps } from 'lucide-react'
import {
  Image,
  BarChart3,
  SlidersHorizontal,
  Tag,
  LayoutGrid,
  Folder,
  FolderPlus,
  ChevronRight,
  ChevronLeft,
  ChevronDown,
  ChevronsRight,
  Clock,
  PanelLeft,
  X,
  Cog,
  Download,
  Zap,
  Check,
  Flag,
  Circle,
  Triangle,
  Target,
  User,
  Star,
  Sparkles,
  Undo2,
  RotateCcw,
  RotateCw,
  ExternalLink,
  Loader2,
  FileDown,
  Plus,
} from 'lucide-react'

// Every icon defaults to 1em so it keeps scaling with the surrounding
// font-size (text-xs / text-sm / text-3xl / etc.), matching how the rest of
// the app already sizes icons — Lucide's own default is a fixed 24px.
function withEmSize(LucideIconComponent: LucideIcon) {
  return function SizedIcon(props: LucideProps) {
    return <LucideIconComponent size="1em" {...props} />
  }
}

export const IconPhoto = withEmSize(Image)
export const IconChartBar = withEmSize(BarChart3)
export const IconSliders = withEmSize(SlidersHorizontal)
export const IconTag = withEmSize(Tag)
export const IconGrid = withEmSize(LayoutGrid)
export const IconFolder = withEmSize(Folder)
export const IconFolderPlus = withEmSize(FolderPlus)
export const IconChevronRight = withEmSize(ChevronRight)
export const IconChevronLeft = withEmSize(ChevronLeft)
export const IconChevronDown = withEmSize(ChevronDown)
export const IconChevronsRight = withEmSize(ChevronsRight)
export const IconClock = withEmSize(Clock)
export const IconSidebar = withEmSize(PanelLeft)
export const IconX = withEmSize(X)
export const IconSettings = withEmSize(Cog)
export const IconDownload = withEmSize(Download)
export const IconZap = withEmSize(Zap)
export const IconCheck = withEmSize(Check)
export const IconFlag = withEmSize(Flag)
export const IconCircle = withEmSize(Circle)
export const IconTriangle = withEmSize(Triangle)
export const IconTarget = withEmSize(Target)
export const IconUser = withEmSize(User)
export const IconStar = withEmSize(Star)
export const IconSparkle = withEmSize(Sparkles)
export const IconUndo = withEmSize(Undo2)
export const IconRotateLeft = withEmSize(RotateCcw)
export const IconRotateRight = withEmSize(RotateCw)
export const IconExternalLink = withEmSize(ExternalLink)
export const IconLoader = withEmSize(Loader2)
export const IconFileDown = withEmSize(FileDown)
export const IconPlus = withEmSize(Plus)
