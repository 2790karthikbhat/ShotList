// Shared filter segmented control, rendered inline inside <StageToolbar> by
// Cull/Analyse/Develop/Caption (Zine doesn't filter). Used to be its own
// standalone bar below the toolbar — folded in here to remove a row of chrome.

import { useStore } from '../store'
import { FILTER_ALL, type FilterState } from '../types'
import { IconCircle, IconCheck, IconX, IconFlag } from './icons'

type IconComponent = (props: React.SVGProps<SVGSVGElement>) => JSX.Element

const FILTER_TABS: { label: string; icon: IconComponent | null; filter: FilterState }[] = [
  { label: 'All',         icon: null,       filter: FILTER_ALL },
  { label: 'Unreviewed',  icon: IconCircle, filter: { showUnreviewed: true,  showPicks: false, showRejects: false, showFlagged: false, minRating: 0 } },
  { label: 'Picks',       icon: IconCheck,  filter: { showUnreviewed: false, showPicks: true,  showRejects: false, showFlagged: false, minRating: 0 } },
  { label: 'Rejects',     icon: IconX,      filter: { showUnreviewed: false, showPicks: false, showRejects: true,  showFlagged: false, minRating: 0 } },
  { label: 'Flagged',     icon: IconFlag,   filter: { showUnreviewed: false, showPicks: false, showRejects: false, showFlagged: true,  minRating: 0 } },
]

export function FilterBar() {
  const session     = useStore(s => s.session)
  const filter      = useStore(s => s.filter)
  const applyFilter = useStore(s => s.applyFilter)

  if (!session) return null

  return (
    <div className="flex items-center gap-0.5 bg-surface/60 rounded-lg p-[3px] flex-shrink-0">
      {FILTER_TABS.map(({ label, icon: Icon, filter: f }) => {
        const active = JSON.stringify(filter) === JSON.stringify(f)
        return (
          <button
            key={label}
            onClick={() => applyFilter(f)}
            className={`flex items-center gap-1 px-3 py-1 rounded-md text-[11.5px] transition-colors duration-100 ${
              active
                ? 'bg-surface text-hi font-medium shadow-sm'
                : 'text-lo hover:text-mid'
            }`}
          >
            {Icon && <Icon className="text-sm" />}
            {label}
          </button>
        )
      })}
    </div>
  )
}
