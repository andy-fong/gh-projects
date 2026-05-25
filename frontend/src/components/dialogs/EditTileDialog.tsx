import { useState } from 'react'
import { X } from 'lucide-react'
import type { Tile } from '../../types'

interface Props {
  tile: Tile
  onConfirm: (update: { title?: string; config?: object }) => void
  onClose: () => void
}

export default function EditTileDialog({ tile, onConfirm, onClose }: Props) {
  const config = JSON.parse(tile.config)
  const [title, setTitle] = useState(tile.title)
  const [command, setCommand] = useState(config.command ?? '')
  const [extractorsJson, setExtractorsJson] = useState(
    JSON.stringify(config.field_extractors ?? {}, null, 2)
  )
  const [jsonError, setJsonError] = useState<string | null>(null)

  function handleSubmit(e: React.FormEvent) {
    e.preventDefault()
    if (tile.tile_type === 'gh_query') {
      try {
        const field_extractors = JSON.parse(extractorsJson || '{}')
        if (typeof field_extractors !== 'object' || Array.isArray(field_extractors))
          throw new Error('Must be a JSON object')
        onConfirm({ title, config: { ...config, command, field_extractors } })
      } catch (err) {
        setJsonError((err as Error).message)
      }
    } else {
      onConfirm({ title, config })
    }
  }

  return (
    <div className="fixed inset-0 bg-black/60 flex items-center justify-center z-50" onClick={onClose}>
      <div className="bg-gray-800 border border-gray-700 rounded-lg p-6 w-[500px] shadow-xl" onClick={e => e.stopPropagation()}>
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-base font-semibold">Edit Tile</h2>
          <button onClick={onClose} className="text-gray-400 hover:text-white"><X className="w-4 h-4" /></button>
        </div>
        <form onSubmit={handleSubmit} className="space-y-4">
          <div>
            <label className="block text-xs text-gray-400 mb-1">Title</label>
            <input value={title} onChange={e => setTitle(e.target.value)} className="w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded text-sm text-white outline-none focus:border-blue-500" />
          </div>
          {tile.tile_type === 'gh_query' && (
            <>
              <div>
                <label className="block text-xs text-gray-400 mb-1">gh command</label>
                <input value={command} onChange={e => setCommand(e.target.value)} className="w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded text-sm font-mono text-white outline-none focus:border-blue-500" />
              </div>
              <div>
                <label className="block text-xs text-gray-400 mb-1">
                  Field extractors override
                  <span className="ml-2 text-gray-600 font-normal">— leave empty to use global settings</span>
                </label>
                <textarea
                  value={extractorsJson}
                  onChange={e => { setExtractorsJson(e.target.value); setJsonError(null) }}
                  rows={3}
                  spellCheck={false}
                  className="w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded text-sm font-mono text-white outline-none focus:border-blue-500 resize-none"
                />
                {jsonError && <p className="text-xs text-red-400 mt-1">{jsonError}</p>}
              </div>
            </>
          )}
          <div className="flex justify-end gap-2 pt-2">
            <button type="button" onClick={onClose} className="px-4 py-2 text-sm text-gray-300 hover:text-white">Cancel</button>
            <button type="submit" className="px-4 py-2 text-sm bg-blue-600 hover:bg-blue-500 rounded text-white">Save</button>
          </div>
        </form>
      </div>
    </div>
  )
}
