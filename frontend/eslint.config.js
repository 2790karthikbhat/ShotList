import js from '@eslint/js'
import globals from 'globals'
import reactHooks from 'eslint-plugin-react-hooks'
import reactRefresh from 'eslint-plugin-react-refresh'
import tseslint from 'typescript-eslint'

export default tseslint.config(
  { ignores: ['dist'] },
  {
    extends: [js.configs.recommended, ...tseslint.configs.recommended],
    files: ['**/*.{ts,tsx}'],
    languageOptions: {
      ecmaVersion: 2020,
      globals: globals.browser,
    },
    plugins: {
      'react-hooks': reactHooks,
      'react-refresh': reactRefresh,
    },
    rules: {
      // Only the long-established hooks rules — eslint-plugin-react-hooks v7
      // bundles the newer React Compiler diagnostics (preserve-manual-memoization,
      // set-state-in-effect, immutability, ...) into `recommended`, which flags
      // ~24 pre-existing, working patterns across the codebase as errors. Revisit
      // adopting those once there's appetite for the broader rewrite they imply.
      'react-hooks/rules-of-hooks': 'error',
      'react-hooks/exhaustive-deps': 'warn',
      'react-refresh/only-export-components': [
        'warn',
        { allowConstantExport: true },
      ],
      '@typescript-eslint/no-unused-vars': ['warn', { argsIgnorePattern: '^_' }],
      // U+202F (narrow no-break space) is used deliberately between a number
      // and its unit (e.g. "50 mm") so it can't line-wrap away from its value.
      'no-irregular-whitespace': ['error', { skipTemplates: true }],
    },
  },
)
