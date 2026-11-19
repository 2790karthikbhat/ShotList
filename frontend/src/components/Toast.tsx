import { useStore } from '../store'

const KIND_STYLES = {
  success: { bar: '#50C378', bg: 'var(--color-toast-success-bg)' },
  error:   { bar: '#D24646', bg: 'var(--color-toast-error-bg)' },
  warning: { bar: '#DCA032', bg: 'var(--color-toast-warning-bg)' },
  info:    { bar: '#4A90D9', bg: 'var(--color-toast-info-bg)' },
}

export function Toast() {
  const { toast, dismissToast } = useStore()
  if (!toast) return null

  const { bar, bg } = KIND_STYLES[toast.kind]

  return (
    <div
      key={toast.id}
      onClick={dismissToast}
      className="absolute bottom-28 left-1/2 -translate-x-1/2 z-50 flex items-center
                 gap-3 px-4 py-2.5 rounded-xl cursor-pointer backdrop-blur-vibrancy
                 shadow-panel text-hi text-xs font-medium
                 animate-[fadeIn_0.15s_ease-out]"
      style={{ background: bg, border: `1px solid ${bar}33`, minWidth: 220, maxWidth: 480 }}
    >
      <div className="w-1 self-stretch rounded-full flex-shrink-0" style={{ background: bar }} />
      <span className="flex-1">{toast.message}</span>
    </div>
  )
}
