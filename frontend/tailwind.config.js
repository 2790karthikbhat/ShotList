/** @type {import('tailwindcss').Config} */
export default {
  content: ['./index.html', './src/**/*.{ts,tsx}'],
  theme: {
    extend: {
      colors: {
        // Theme-aware chrome — RGB-triplet vars defined in :root /
        // [data-theme="light"] (index.css), so bg-x/50-style opacity
        // modifiers keep working: rgb(var(--color-x-rgb) / <alpha-value>).
        bg:       'rgb(var(--color-bg-rgb) / <alpha-value>)',
        panel:    'rgb(var(--color-panel-rgb) / <alpha-value>)',
        surface:  'rgb(var(--color-surface-rgb) / <alpha-value>)',
        border:   'rgb(var(--color-border-rgb) / <alpha-value>)',
        accent:   'rgb(var(--color-accent-rgb) / <alpha-value>)',
        hi:       'rgb(var(--color-hi-rgb) / <alpha-value>)',
        mid:      'rgb(var(--color-mid-rgb) / <alpha-value>)',
        lo:       'rgb(var(--color-lo-rgb) / <alpha-value>)',
        // Semantic cull colours — theme-independent by design
        pick:     '#50C378',
        reject:   '#D24646',
        flag:     '#DCA032',
      },
      fontFamily: {
        mono: ['ui-monospace', 'SFMono-Regular', 'Menlo', 'monospace'],
      },
      borderRadius: {
        xl:  '14px',
        '2xl': '18px',
      },
      backdropBlur: {
        vibrancy: '28px',
      },
      boxShadow: {
        card:  '0 12px 28px -10px rgb(0 0 0 / 0.35)',
        panel: '0 24px 56px -20px rgb(0 0 0 / 0.45)',
      },
    },
  },
  plugins: [],
}
