import { useState } from 'react'
import { X } from 'lucide-react'
import { useSettings } from '../../context/SettingsContext'
import { DEFAULT_FIELD_EXTRACTORS } from '../../lib/fieldExtractors'

interface Props {
  onClose: () => void
}

export default function SettingsDialog({ onClose }: Props) {
  const { settings, updateSettings } = useSettings()

  const [extractorsJson, setExtractorsJson] = useState(
    JSON.stringify(settings.field_extractors, null, 2)
  )
  const [jsonError, setJsonError] = useState<string | null>(null)

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
              Built-in defaults (always applied unless overridden):
              <code className="ml-1 text-gray-500">{defaults}</code>
            </p>
            <p className="text-xs text-gray-600 mt-1">
              Per-tile overrides in the tile edit dialog take precedence over these.
            </p>
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
