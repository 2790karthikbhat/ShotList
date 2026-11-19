// Merged sidebar — combines the former StageNav (stage-switcher rail) and
// FolderSidebar (folder browser) into one bordered, translucent panel.
// Always expanded (no collapse toggle) — a collapse control here crowded the
// traffic-light cluster it sits directly under, so the sidebar just stays put.

import { useLayoutEffect, useRef, useState, useEffect, useCallback } from 'react'
import { useStore } from '../store'
import * as ipc from '../lib/ipc'
import type { Stage, DirEntry, RootLocation } from '../types'
import {
  IconPhoto, IconChartBar, IconSliders, IconTag, IconGrid,
  IconFolderPlus, IconFolder, IconChevronRight, IconClock, IconX,
} from './icons'

interface NavItem {
  stage: Stage
  icon:  (props: React.SVGProps<SVGSVGElement>) => JSX.Element
  label: string
}

const NAV: NavItem[] = [
  { stage: 'cull',    icon: IconPhoto,    label: 'Cull'    },
  { stage: 'analyse', icon: IconChartBar, label: 'Analyse' },
  { stage: 'develop', icon: IconSliders,  label: 'Develop' },
  { stage: 'caption', icon: IconTag,      label: 'Caption' },
  { stage: 'zine',    icon: IconGrid,     label: 'Zine'    },
]

const SIDEBAR_W  = 220
const PILL_INSET = 8

export function Sidebar() {
  const { stage, setStage, session } = useStore()
  const importFolder = useStore(s => s.importFolder)
  const openFolderAt = useStore(s => s.openFolderAt)

  const [recents, setRecents] = useState<string[]>([])
  const [roots, setRoots] = useState<RootLocation[]>([])
  const [quickLookPath, setQuickLookPath] = useState<string | null>(null)

  useEffect(() => {
    ipc.getSettings().then(s => setRecents(s.recentFolders)).catch(() => {})
  }, [session])

  useEffect(() => {
    ipc.getRootLocations().then(setRoots).catch(() => {})
  }, [])

  const activeIndex = NAV.findIndex(n => n.stage === stage)
  const btnRefs = useRef<(HTMLButtonElement | null)[]>([])
  const [pill, setPill] = useState<{ top: number; height: number } | null>(null)

  useLayoutEffect(() => {
    const el = btnRefs.current[activeIndex]
    if (el) setPill({ top: el.offsetTop, height: el.offsetHeight })
  }, [activeIndex])

  return (
    <aside
      className="flex flex-col vibrancy-panel border-r border-border flex-shrink-0 overflow-hidden"
      style={{ width: SIDEBAR_W }}
    >
      {/* 28px overlay title-bar strip — traffic lights own this area visually. */}
      <div data-tauri-drag-region className="w-full h-7 flex-shrink-0" />

      {/* Stage nav */}
      <div className="px-2 pt-2 relative flex-shrink-0 pb-1">
        {pill && (
          <div
            className="absolute rounded-lg bg-accent/10 shadow-sm transition-[top] duration-200 ease-out overflow-hidden"
            style={{ top: pill.top, height: pill.height, left: PILL_INSET, right: PILL_INSET }}
          >
            <div className="absolute left-0 top-1.5 bottom-1.5 w-[3px] rounded-full bg-accent" />
          </div>
        )}
        {NAV.map(({ stage: s, icon: Icon, label }, i) => {
          const active  = stage === s
          const enabled = s === 'cull' || !!session
          return (
            <button
              key={s}
              ref={el => { btnRefs.current[i] = el }}
              onClick={() => enabled && setStage(s)}
              title={label}
              className={`
                relative w-full h-[40px] flex items-center justify-start px-3 gap-3 rounded-lg text-2xl mb-0.5
                transition-colors duration-100
                ${active
                  ? 'text-accent font-medium'
                  : enabled
                    ? 'text-hi hover:text-accent'
                    : 'text-lo cursor-not-allowed'}
              `}
            >
              <Icon className="flex-shrink-0" />
              <span className="text-sm font-medium truncate">{label}</span>
            </button>
          )
        })}
      </div>

      {/* Folder tree — separated from the nav by whitespace, not a rule; a hairline
          here would sit inconsistently next to the sidebar's one full-bleed border
          (the one separating it from the main content). Scrolls independently;
          "Open folder…" stays pinned below it. */}
      <div className="flex-1 overflow-y-auto px-2 pt-4 pb-2 min-h-0">
        {recents.length > 0 && (
          <>
            <SectionLabel>Recents</SectionLabel>
            {recents.map(path => (
              <FlatRow
                key={path}
                path={path}
                onOpen={() => openFolderAt(path)}
                onRemove={() => {
                  setRecents(r => r.filter(p => p !== path))
                  ipc.removeRecentFolder(path).catch(() => {})
                }}
              />
            ))}
          </>
        )}

        <SectionLabel>Locations</SectionLabel>
        {roots.map(root => (
          <TreeNode
            key={root.path}
            name={root.label}
            path={root.path}
            depth={0}
            onOpen={openFolderAt}
            onQuickLook={setQuickLookPath}
          />
        ))}
      </div>

      <div className="px-2 pb-3 flex-shrink-0">
        <button
          onClick={importFolder}
          className="w-full flex items-center gap-2 px-2.5 py-2 rounded-lg border border-dashed border-border text-mid hover:text-hi hover:border-mid text-xs transition-colors"
        >
          <IconFolderPlus className="flex-shrink-0 text-base" />
          Open folder…
        </button>
      </div>

      {quickLookPath && (
        <QuickLook path={quickLookPath} onClose={() => setQuickLookPath(null)} />
      )}
    </aside>
  )
}

function parentDir(path: string): string {
  const parts = path.split('/')
  parts.pop()
  return parts.join('/') || '/'
}

function SectionLabel({ children }: { children: React.ReactNode }) {
  return (
    <div className="text-[10px] uppercase tracking-widest text-lo px-2 pt-4 pb-1.5 first:pt-0">
      {children}
    </div>
  )
}

function FlatRow({
  path,
  onOpen,
  onRemove,
}: {
  path: string
  onOpen: () => void
  onRemove: () => void
}) {
  const name = path.split('/').filter(Boolean).pop() || path
  return (
    <div className="group w-full flex items-center gap-1 rounded-lg hover:bg-surface transition-colors duration-100">
      <button
        onClick={onOpen}
        title={path}
        className="flex-1 min-w-0 flex items-center gap-1.5 px-2 py-1.5 text-mid group-hover:text-hi text-xs text-left"
      >
        <IconClock className="flex-shrink-0 text-lo" />
        <span className="truncate">{name}</span>
      </button>
      <button
        onClick={e => { e.stopPropagation(); onRemove() }}
        title="Remove from Recents"
        className="flex-shrink-0 px-1.5 py-1.5 mr-0.5 text-lo hover:text-hi opacity-0 group-hover:opacity-100 transition-opacity"
      >
        <IconX />
      </button>
    </div>
  )
}

function TreeNode({
  name,
  path,
  depth,
  onOpen,
  onQuickLook,
}: {
  name: string
  path: string
  depth: number
  onOpen: (path: string) => void
  onQuickLook: (path: string) => void
}) {
  const [expanded, setExpanded] = useState(false)
  const [children, setChildren] = useState<DirEntry[] | null>(null)
  const [loading, setLoading] = useState(false)

  const toggle = useCallback(() => {
    if (!expanded && children === null) {
      setLoading(true)
      ipc
        .listDirEntries(path)
        .then(setChildren)
        .catch(() => setChildren([]))
        .finally(() => setLoading(false))
    }
    setExpanded(e => !e)
  }, [expanded, children, path])

  const indent = 8 + depth * 14

  return (
    <div>
      <div className="flex items-center gap-1 rounded-lg hover:bg-surface transition-colors duration-100" style={{ paddingLeft: indent }}>
        <button
          onClick={toggle}
          title={expanded ? 'Collapse' : 'Expand'}
          className="flex-shrink-0 py-1.5 text-lo hover:text-hi"
        >
          <IconChevronRight
            className={`transition-transform duration-150 ${expanded ? 'rotate-90' : ''}`}
          />
        </button>
        <button
          onClick={() => onOpen(path)}
          title={`Open ${path} for culling`}
          className="flex-1 min-w-0 flex items-center gap-1 py-1.5 text-mid hover:text-hi text-xs text-left"
        >
          <IconFolder className="flex-shrink-0 text-lo" />
          <span className="truncate">{name}</span>
        </button>
      </div>
      {expanded && (
        <div>
          {loading && (
            <div className="text-[11px] text-lo py-1" style={{ paddingLeft: indent + 14 }}>
              Loading…
            </div>
          )}
          {!loading && children?.length === 0 && (
            <div className="text-[11px] text-lo py-1" style={{ paddingLeft: indent + 14 }}>
              Empty folder
            </div>
          )}
          {!loading &&
            children?.map(c =>
              c.kind === 'folder' ? (
                <TreeNode
                  key={c.path}
                  name={c.name}
                  path={c.path}
                  depth={depth + 1}
                  onOpen={onOpen}
                  onQuickLook={onQuickLook}
                />
              ) : (
                <ImageRow
                  key={c.path}
                  name={c.name}
                  path={c.path}
                  depth={depth + 1}
                  onQuickLook={onQuickLook}
                />
              ),
            )}
        </div>
      )}
    </div>
  )
}

function ImageRow({
  name,
  path,
  depth,
  onQuickLook,
}: {
  name: string
  path: string
  depth: number
  onQuickLook: (path: string) => void
}) {
  // +14 aligns under folder names, in the slot a chevron would occupy.
  const indent = 8 + depth * 14 + 14
  return (
    <button
      onClick={() => onQuickLook(path)}
      title={`Quick Look ${name}`}
      className="w-full flex items-center gap-1 py-1.5 rounded-lg text-mid hover:bg-surface hover:text-hi text-xs text-left transition-colors duration-100"
      style={{ paddingLeft: indent }}
    >
      <IconPhoto className="flex-shrink-0 text-lo" />
      <span className="truncate">{name}</span>
    </button>
  )
}

function QuickLook({ path, onClose }: { path: string; onClose: () => void }) {
  const openFolderAtImage = useStore(s => s.openFolderAtImage)
  const [src, setSrc] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)
  const name = path.split('/').filter(Boolean).pop() || path

  useEffect(() => {
    setSrc(null)
    setError(null)
    ipc
      .previewFile(path)
      .then(setSrc)
      .catch(e => setError(e instanceof Error ? e.message : String(e)))
  }, [path])

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose()
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [onClose])

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/70"
      onClick={e => { if (e.target === e.currentTarget) onClose() }}
    >
      <div className="vibrancy-panel border border-border rounded-2xl w-[560px] max-w-[90vw] shadow-panel flex flex-col overflow-hidden">
        <div className="flex items-center justify-between px-4 py-2.5 border-b border-border flex-shrink-0">
          <span className="text-hi text-xs font-medium truncate pr-2">{name}</span>
          <button onClick={onClose} className="text-mid hover:text-hi flex-shrink-0">
            <IconX />
          </button>
        </div>

        <div className="bg-bg flex items-center justify-center" style={{ height: 380 }}>
          {error ? (
            <span className="text-lo text-xs px-4 text-center">{error}</span>
          ) : src ? (
            <img src={src} alt={name} className="max-w-full max-h-full object-contain" draggable={false} />
          ) : (
            <span className="text-lo text-xs animate-pulse">Loading…</span>
          )}
        </div>

        <div className="flex justify-end gap-2 px-4 py-3 border-t border-border flex-shrink-0">
          <button
            onClick={onClose}
            className="px-3 py-1.5 rounded-lg text-xs text-mid hover:text-hi hover:bg-surface transition-colors"
          >
            Close
          </button>
          <button
            onClick={() => {
              openFolderAtImage(parentDir(path), path)
              onClose()
            }}
            className="px-3 py-1.5 rounded-lg text-xs bg-accent text-black font-medium hover:brightness-110 transition-all"
          >
            Open in Develop
          </button>
        </div>
      </div>
    </div>
  )
}
