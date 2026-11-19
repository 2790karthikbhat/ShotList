import { useSortable } from '@dnd-kit/sortable'
import { CSS } from '@dnd-kit/utilities'

import type { ZinePage } from '../../types'
import { TEMPLATE_LABELS } from '../../types'
import { IconX, IconPhoto } from '../../components/icons'

export function SortablePage({
  page, isSelected, thumb, onSelect, onRemove,
}: {
  page: ZinePage; isSelected: boolean; thumb: string | undefined
  onSelect: () => void; onRemove: () => void
}) {
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } =
    useSortable({ id: page.id })

  return (
    <div
      ref={setNodeRef}
      style={{ transform: CSS.Transform.toString(transform), transition, opacity: isDragging ? 0.4 : 1, flexShrink: 0 }}
      className="relative group"
    >
      <div
        {...attributes} {...listeners}
        onClick={onSelect}
        style={{ cursor: 'grab' }}
        className={`w-24 h-32 rounded-xl border-2 overflow-hidden transition-all
          ${isSelected
            ? 'border-amber-400 shadow-[0_0_0_2px_rgba(251,191,36,0.25)] -translate-y-1'
            : 'border-transparent hover:border-lo hover:-translate-y-0.5'}`}
      >
        {thumb
          ? <img src={thumb} alt="" className="w-full h-full object-cover" draggable={false} />
          : <div className="w-full h-full bg-surface flex flex-col items-center justify-center gap-1">
              <IconPhoto size={16} className="text-lo" />
              <span className="text-[9px] text-lo">
                {page.template === 'titlePage' ? 'Title Page' : 'No image'}
              </span>
            </div>
        }
        {/* Template label */}
        <div className="absolute bottom-0 inset-x-0 bg-black/60 text-[9px] text-hi px-1 py-0.5 truncate">
          {TEMPLATE_LABELS[page.template]}
        </div>
        {/* Position number */}
        <div className="absolute top-1 left-1 bg-black/60 text-[9px] text-white rounded px-1">
          {page.position + 1}
        </div>
      </div>

      {/* Remove button — shown on hover */}
      <button
        onClick={e => { e.stopPropagation(); onRemove() }}
        className="absolute -top-1.5 -right-1.5 w-4 h-4 rounded-full bg-red-600 hover:bg-red-500
          text-white items-center justify-center hidden group-hover:flex transition-colors z-10"
        title="Remove page"
      >
        <IconX size={9} strokeWidth={3} />
      </button>
    </div>
  )
}
