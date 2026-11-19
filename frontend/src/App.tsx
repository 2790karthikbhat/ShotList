import { useEffect, useState } from 'react'
import { useStore } from './store'
import { Sidebar } from './components/Sidebar'
import { Toast } from './components/Toast'
import { CullStage }    from './stages/CullStage'
import { AnalyseStage } from './stages/AnalyseStage'
import { DevelopStage }  from './stages/DevelopStage'
import { CaptionStage }  from './stages/CaptionStage'
import { ZineStage }     from './stages/ZineStage'
import * as ipc from './lib/ipc'
import { applyTheme } from './lib/theme'
import type { AppSettings } from './types'
import { IconX } from './components/icons'

// ── Settings modal ────────────────────────────────────────────────────────────

function SettingsModal() {
  const settingsOpen  = useStore(s => s.settingsOpen)
  const closeSettings = useStore(s => s.closeSettings)
  const showToast     = useStore(s => s.showToast)

  const [form, setForm] = useState<AppSettings>({
    anthropicKey:  '',
    openaiKey:     '',
    autoAdvance:   true,
    showRejects:   true,
    aiProvider:    'anthropic',
    lmstudioUrl:   'http://localhost:1234/v1',
    lmstudioModel: '',
    recentFolders: [],
    theme: 'dark',
  })
  const [loading, setLoading] = useState(false)
  const [testing, setTesting] = useState(false)

  useEffect(() => {
    if (!settingsOpen) return
    ipc.getSettings().then(s => setForm(s)).catch(() => {})
  }, [settingsOpen])

  if (!settingsOpen) return null

  const save = async () => {
    setLoading(true)
    try {
      await ipc.saveSettings(form)
      applyTheme(form.theme)
      showToast('Settings saved', 'success')
      closeSettings()
    } catch (e) {
      showToast(`Failed to save settings: ${e}`, 'error')
    } finally {
      setLoading(false)
    }
  }

  const testConnection = async () => {
    setTesting(true)
    try {
      const models = await ipc.testLmStudioConnection(form.lmstudioUrl)
      if (models.length === 0) {
        showToast('Connected — no models loaded yet', 'warning')
      } else {
        const preview = models.slice(0, 3).join(', ') + (models.length > 3 ? '…' : '')
        showToast(`Connected — ${models.length} model${models.length === 1 ? '' : 's'}: ${preview}`, 'success')
      }
    } catch (e) {
      showToast(`Cannot connect: ${e}`, 'error')
    } finally {
      setTesting(false)
    }
  }

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/70"
      onClick={(e) => { if (e.target === e.currentTarget) closeSettings() }}
    >
      <div className="vibrancy-panel border border-border rounded-2xl w-[460px] shadow-panel">
        {/* Header */}
        <div className="flex items-center justify-between px-5 py-4 border-b border-border">
          <span className="text-hi font-medium text-sm">Settings</span>
          <button
            onClick={closeSettings}
            className="text-mid hover:text-hi transition-colors"
          ><IconX /></button>
        </div>

        {/* Body */}
        <div className="px-5 py-5 space-y-5">

          {/* Appearance */}
          <div className="space-y-2">
            <label className="text-hi text-xs block">Appearance</label>
            <div className="flex gap-2">
              {(['dark', 'light', 'system'] as const).map(t => (
                <button
                  key={t}
                  onClick={() => setForm(f => ({ ...f, theme: t }))}
                  className={`px-3 py-1.5 rounded-lg text-xs border transition-colors capitalize ${
                    form.theme === t
                      ? 'bg-accent text-black border-accent font-medium'
                      : 'border-border text-mid hover:text-hi hover:border-mid'
                  }`}
                >
                  {t}
                </button>
              ))}
            </div>
          </div>

          {/* AI provider selector */}
          <div className="space-y-2">
            <label className="text-hi text-xs block">AI provider</label>
            <div className="flex gap-2">
              {(['anthropic', 'lmstudio'] as const).map(p => (
                <button
                  key={p}
                  onClick={() => setForm(f => ({ ...f, aiProvider: p }))}
                  className={`px-3 py-1.5 rounded-lg text-xs border transition-colors ${
                    form.aiProvider === p
                      ? 'bg-accent text-black border-accent font-medium'
                      : 'border-border text-mid hover:text-hi hover:border-mid'
                  }`}
                >
                  {p === 'anthropic' ? 'Anthropic Claude' : 'LM Studio (local)'}
                </button>
              ))}
            </div>
          </div>

          {/* Anthropic section */}
          {form.aiProvider === 'anthropic' && (
            <Field label="Anthropic API key" hint="Used for auto-develop, captions, and AI sequencing">
              <input
                type="password"
                value={form.anthropicKey}
                onChange={e => setForm(f => ({ ...f, anthropicKey: e.target.value }))}
                placeholder="sk-ant-…"
                className="input"
              />
            </Field>
          )}

          {/* LM Studio section */}
          {form.aiProvider === 'lmstudio' && (
            <div className="space-y-3">
              <Field label="LM Studio server URL" hint="Default: http://localhost:1234/v1">
                <input
                  type="text"
                  value={form.lmstudioUrl}
                  onChange={e => setForm(f => ({ ...f, lmstudioUrl: e.target.value }))}
                  placeholder="http://localhost:1234/v1"
                  className="input"
                />
              </Field>
              <Field label="Model name" hint="Exact model ID as shown in LM Studio (e.g. llava-1.5-7b-hf)">
                <input
                  type="text"
                  value={form.lmstudioModel}
                  onChange={e => setForm(f => ({ ...f, lmstudioModel: e.target.value }))}
                  placeholder="llava-1.5-7b-hf"
                  className="input"
                />
              </Field>
              <button
                onClick={testConnection}
                disabled={testing}
                className="px-3 py-1.5 rounded-lg text-xs border border-border text-mid
                           hover:text-hi hover:border-mid disabled:opacity-50 transition-colors"
              >
                {testing ? 'Testing…' : 'Test connection'}
              </button>
              <p className="text-lo text-[11px]">
                Caption and auto-develop require a multimodal model (e.g. LLaVA, Moondream2).
                AI sequencing works with any text model.
              </p>
            </div>
          )}

          <div className="flex items-center gap-3">
            <Toggle
              checked={form.autoAdvance}
              onChange={v => setForm(f => ({ ...f, autoAdvance: v }))}
            />
            <div>
              <p className="text-hi text-xs">Auto-advance after cull decision</p>
              <p className="text-lo text-[11px]">Move to next image automatically on P / X</p>
            </div>
          </div>

          <div className="flex items-center gap-3">
            <Toggle
              checked={form.showRejects}
              onChange={v => setForm(f => ({ ...f, showRejects: v }))}
            />
            <div>
              <p className="text-hi text-xs">Show rejects in filmstrip</p>
              <p className="text-lo text-[11px]">Dimmed but still visible; useful for recovering choices</p>
            </div>
          </div>
        </div>

        {/* Footer */}
        <div className="flex justify-end gap-2 px-5 py-4 border-t border-border">
          <button
            onClick={closeSettings}
            className="px-4 py-1.5 rounded-lg text-xs text-mid hover:text-hi hover:bg-surface transition-colors"
          >
            Cancel
          </button>
          <button
            onClick={save}
            disabled={loading}
            className="px-4 py-1.5 rounded-lg text-xs bg-accent text-black font-medium
                       hover:brightness-110 disabled:opacity-50 transition-all"
          >
            {loading ? 'Saving…' : 'Save'}
          </button>
        </div>
      </div>
    </div>
  )
}

function Field({ label, hint, children }: { label: string; hint?: string; children: React.ReactNode }) {
  return (
    <div className="space-y-1">
      <label className="text-hi text-xs block">{label}</label>
      {hint && <p className="text-lo text-[11px]">{hint}</p>}
      {children}
    </div>
  )
}

function Toggle({ checked, onChange }: { checked: boolean; onChange: (v: boolean) => void }) {
  return (
    <button
      onClick={() => onChange(!checked)}
      className={`relative w-9 h-5 rounded-full flex-shrink-0 transition-colors duration-200 ${
        checked ? 'bg-accent' : 'bg-surface'
      }`}
    >
      <span
        className="absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-white shadow transition-transform duration-200"
        style={{ transform: checked ? 'translateX(16px)' : 'translateX(0)' }}
      />
    </button>
  )
}

// ── Stage router ──────────────────────────────────────────────────────────────

function StageContent() {
  const stage = useStore(s => s.stage)

  const stageEl = (() => {
    switch (stage) {
      case 'cull':    return <CullStage />
      case 'analyse': return <AnalyseStage />
      case 'develop': return <DevelopStage />
      case 'caption': return <CaptionStage />
      case 'zine':    return <ZineStage />
    }
  })()

  // key={stage} forces a remount on stage change so the fade-in animation
  // replays; the old view has no exit transition, just the incoming one.
  return (
    <div key={stage} className="flex flex-col flex-1 overflow-hidden stage-fade-in">
      {stageEl}
    </div>
  )
}

// ── App root ──────────────────────────────────────────────────────────────────

export default function App() {
  const initListeners = useStore(s => s.initListeners)

  useEffect(() => {
    let cleanup: (() => void) | undefined

    initListeners().then(fn => { cleanup = fn }).catch(console.error)

    return () => { cleanup?.() }
  }, [initListeners])

  useEffect(() => {
    ipc.getSettings().then(s => applyTheme(s.theme)).catch(() => {})
  }, [])

  return (
    <div className="flex h-screen w-screen overflow-hidden bg-bg text-hi">
      <Sidebar />

      {/* Main content area */}
      <main className="flex flex-col flex-1 overflow-hidden relative">
        <StageContent />
        <Toast />
      </main>

      <SettingsModal />
    </div>
  )
}
