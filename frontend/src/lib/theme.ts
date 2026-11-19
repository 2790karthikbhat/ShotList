// Applies the app's Dark/Light/System theme setting to the document root.
// 'system' resolves via matchMedia and stays live-reactive to OS changes
// while it remains the active mode.

import type { AppSettings } from '../types'

let systemListenerCleanup: (() => void) | null = null

export function applyTheme(theme: AppSettings['theme']) {
  systemListenerCleanup?.()
  systemListenerCleanup = null

  const set = (resolved: 'dark' | 'light') => {
    document.documentElement.dataset.theme = resolved
  }

  if (theme === 'system') {
    const mql = window.matchMedia('(prefers-color-scheme: dark)')
    const update = () => set(mql.matches ? 'dark' : 'light')
    update()
    mql.addEventListener('change', update)
    systemListenerCleanup = () => mql.removeEventListener('change', update)
  } else {
    set(theme)
  }
}
