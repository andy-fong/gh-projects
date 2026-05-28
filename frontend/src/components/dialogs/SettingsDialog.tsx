import { useRef, useState } from 'react'
import { X } from 'lucide-react'
import { useSettings } from '../../context/SettingsContext'
import { DEFAULT_FIELD_EXTRACTORS } from '../../lib/fieldExtractors'
import { api } from '../../api/client'

interface Props {
  onClose: () => void
}

export default function SettingsDialog({ onClose }: Props) {
  const { settings, updateSettings } = useSettings()

  const [extractorsJson, setExtractorsJson] = useState(
    JSON.stringify(settings.field_extractors, null, 2)
  )
  const [jsonError, setJsonError] = useState<string | null>(null)
  const [restoreError, setRestoreError] = useState<string | null>(null)
  const [restoreSuccess, setRestoreSuccess] = useState(false)
  const [exporting, setExporting] = useState(false)
  const [restoring, setRestoring] = useState(false)
  const fileInputRef = useRef<HTMLInputElement>(null)

  function handleSubmit(e: React.FormEvent) {
    e.preventDefault()
    try {
      const parsed = JSON.parse(extractorsJson || '{}')
      if (typeof parsed !== 'object' || Array.isArray(parsed)) throw new Error('Must be a JSON object')
      updateSettings({ field_extractors: parsed })
      setJsonError(null)
      onClose()
    } catch (err) {
      setJsonError((err as Error).message)
    }
  }

  async function handleExport() {
    setExporting(true)
    try {
      const data = await api.backup.export()
      // Enrich each tile with its localStorage column state
      for (const dashboard of data.dashboards) {
        for (const tile of dashboard.tiles) {
          const hidden = localStorage.getItem(`gh-tile-hidden-${tile.id}`)
          const order = localStorage.getItem(`gh-tile-col-order-${tile.id}`)
          if (hidden !== null) (tile as Record<string, unknown>).hidden_cols = JSON.parse(hidden)
          if (order !== null) (tile as Record<string, unknown>).col_order = JSON.parse(order)
        }
      }
      const now = new Date()
      const pad = (n: number) => String(n).padStart(2, '0')
      const filename = `gh_project_backup_${now.getFullYear()}${pad(now.getMonth() + 1)}${pad(now.getDate())}-${pad(now.getHours())}${pad(now.getMinutes())}.json`
      const blob = new Blob([JSON.stringify(data, null, 2)], { type: 'application/json' })
      const url = URL.createObjectURL(blob)
      const a = document.createElement('a')
      a.href = url
      a.download = filename
      a.click()
      URL.revokeObjectURL(url)
    } finally {
      setExporting(false)
    }
  }

  async function handleRestoreFile(e: React.ChangeEvent<HTMLInputElement>) {
    const file = e.target.files?.[0]
    if (!file) return
    setRestoring(true)
    setRestoreError(null)
    setRestoreSuccess(false)
    try {
      const text = await file.text()
      const data = JSON.parse(text) as {
        dashboards: { tiles: { hidden_cols?: string[]; col_order?: string[] }[] }[]
        notes: unknown[]
      }
      const result = await api.backup.restore(data)

      // Clear all stale tile localStorage keys
      for (const key of Object.keys(localStorage)) {
        if (key.startsWith('gh-tile-hidden-') || key.startsWith('gh-tile-col-order-')) {
          localStorage.removeItem(key)
        }
      }

      // Write localStorage for restored tiles using new IDs returned by the server
      for (let di = 0; di < result.dashboards.length; di++) {
        const tileIds = result.dashboards[di].tile_ids
        const tiles = data.dashboards[di]?.tiles ?? []
        for (let ti = 0; ti < tileIds.length; ti++) {
          const newId = tileIds[ti]
          const tile = tiles[ti]
          if (tile?.hidden_cols) localStorage.setItem(`gh-tile-hidden-${newId}`, JSON.stringify(tile.hidden_cols))
          if (tile?.col_order) localStorage.setItem(`gh-tile-col-order-${newId}`, JSON.stringify(tile.col_order))
        }
      }

      setRestoreSuccess(true)
    } catch (err) {
      setRestoreError((err as Error).message)
    } finally {
      setRestoring(false)
      if (fileInputRef.current) fileInputRef.current.value = ''
    }
  }

  const defaults = JSON.stringify(DEFAULT_FIELD_EXTRACTORS, null, 2)

  return (
    <div className="fixed inset-0 bg-black/60 flex items-center justify-center z-50" onClick={onClose}>
      <div className="bg-gray-800 border border-gray-700 rounded-lg p-6 w-[520px] shadow-xl" onClick={e => e.stopPropagation()}>
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-base font-semibold">Global Settings</h2>
          <button onClick={onClose} className="text-gray-400 hover:text-white"><X className="w-4 h-4" /></button>
        </div>

        <form onSubmit={handleSubmit} className="space-y-4">
          <div>
            <label className="block text-xs text-gray-400 mb-1">
              Field extractors
              <span className="ml-2 text-gray-600 normal-case font-normal">— maps column → sub-field to display</span>
            </label>
            <textarea
              value={extractorsJson}
              onChange={e => { setExtractorsJson(e.target.value); setJsonError(null) }}
              rows={6}
              spellCheck={false}
              className="w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded text-sm font-mono text-white outline-none focus:border-blue-500 resize-none"
            />
            {jsonError && <p className="text-xs text-red-400 mt-1">{jsonError}</p>}
            <p className="text-xs text-gray-600 mt-1">
              Use <code className="text-gray-500">|</code> for fallbacks: <code className="text-gray-500">"name|login"</code> tries <code className="text-gray-500">name</code> first, falls back to <code className="text-gray-500">login</code> if empty.
            </p>
            <p className="text-xs text-gray-600 mt-1">
              Built-in defaults (always applied unless overridden):
              <code className="ml-1 text-gray-500">{defaults}</code>
            </p>
            <p className="text-xs text-gray-600 mt-1">
              Per-tile overrides in the tile edit dialog take precedence over these.
            </p>
          </div>

          <div className="border-t border-gray-700 pt-4">
            <p className="text-xs text-gray-400 mb-3">Backup &amp; Restore</p>
            <div className="flex gap-2">
              <button
                type="button"
                onClick={handleExport}
                disabled={exporting}
                className="px-3 py-1.5 text-sm bg-gray-700 hover:bg-gray-600 disabled:opacity-50 rounded text-white"
              >
                {exporting ? 'Exporting…' : 'Export backup'}
              </button>
              <label className={`px-3 py-1.5 text-sm rounded text-white cursor-pointer ${restoring ? 'bg-gray-700 opacity-50' : 'bg-gray-700 hover:bg-gray-600'}`}>
                {restoring ? 'Restoring…' : 'Restore backup'}
                <input
                  ref={fileInputRef}
                  type="file"
                  accept=".json"
                  className="hidden"
                  disabled={restoring}
                  onChange={handleRestoreFile}
                />
              </label>
            </div>
            {restoreError && <p className="text-xs text-red-400 mt-2">{restoreError}</p>}
            {restoreSuccess && <p className="text-xs text-green-400 mt-2">Restore successful — reload the page to see your data.</p>}
          </div>

          <div className="flex justify-end gap-2 pt-2">
            <button type="button" onClick={onClose} className="px-4 py-2 text-sm text-gray-300 hover:text-white">Cancel</button>
            <button type="submit" className="px-4 py-2 text-sm bg-blue-600 hover:bg-blue-500 rounded text-white">Save</button>
          </div>
        </form>
      </div>
    </div>
  )
}
