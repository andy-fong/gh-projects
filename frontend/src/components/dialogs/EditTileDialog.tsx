import { useState } from 'react'
import { X, Plus, Trash2 } from 'lucide-react'
import type { Tile } from '../../types'

interface Props {
  tile: Tile
  onConfirm: (update: { title?: string; config?: object }) => void
  onClose: () => void
}

export default function EditTileDialog({ tile, onConfirm, onClose }: Props) {
  const config = JSON.parse(tile.config)
  const [title, setTitle] = useState(tile.title)
  const [commands, setCommands] = useState<string[]>(
    config.commands?.length ? config.commands : [config.command ?? '']
  )
  const [extractorsJson, setExtractorsJson] = useState(
    JSON.stringify(config.field_extractors ?? {}, null, 2)
  )
  const [variablesJson, setVariablesJson] = useState(
    config.variables && Object.keys(config.variables).length
      ? JSON.stringify(config.variables, null, 2)
      : ''
  )
  const [jsonError, setJsonError] = useState<string | null>(null)

  function setCommand(i: number, val: string) {
    setCommands(prev => prev.map((c, idx) => idx === i ? val : c))
  }

  function addCommand() {
    setCommands(prev => [...prev, ''])
  }

  function removeCommand(i: number) {
    setCommands(prev => prev.filter((_, idx) => idx !== i))
  }

  function handleSubmit(e: React.FormEvent) {
    e.preventDefault()
    if (tile.tile_type === 'gh_query') {
      try {
        const field_extractors = JSON.parse(extractorsJson || '{}')
        if (typeof field_extractors !== 'object' || Array.isArray(field_extractors))
          throw new Error('Field extractors must be a JSON object')
        const variables = variablesJson.trim() ? JSON.parse(variablesJson) : {}
        if (typeof variables !== 'object' || Array.isArray(variables))
          throw new Error('Variables must be a JSON object')
        const filled = commands.filter(c => c.trim())
        onConfirm({ title, config: { ...config, command: filled[0], commands: filled, field_extractors, variables } })
      } catch (err) {
        setJsonError((err as Error).message)
      }
    } else {
      onConfirm({ title, config })
    }
  }

  return (
    <div className="fixed inset-0 bg-black/60 flex items-center justify-center z-50" onClick={onClose}>
      <div className="bg-gray-800 border border-gray-700 rounded-lg p-6 w-[560px] shadow-xl" onClick={e => e.stopPropagation()}>
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-base font-semibold">Edit Tile</h2>
          <button onClick={onClose} className="text-gray-400 hover:text-white"><X className="w-4 h-4" /></button>
        </div>
        <form onSubmit={handleSubmit} className="space-y-4">
          <div>
            <label className="block text-xs text-gray-400 mb-1">Title</label>
            <input value={title} onChange={e => setTitle(e.target.value)}
              className="w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded text-sm text-white outline-none focus:border-blue-500" />
          </div>
          {tile.tile_type === 'gh_query' && (
            <>
              <div>
                <label className="block text-xs text-gray-400 mb-1">
                  gh command(s)
                  <span className="ml-2 text-gray-600 font-normal">— multiple commands are merged into one table</span>
                </label>
                <div className="space-y-2">
                  {commands.map((cmd, i) => (
                    <div key={i} className="flex gap-2 items-center">
                      <input
                        value={cmd}
                        onChange={e => setCommand(i, e.target.value)}
                        className="flex-1 px-3 py-2 bg-gray-700 border border-gray-600 rounded text-sm font-mono text-white outline-none focus:border-blue-500"
                      />
                      {commands.length > 1 && (
                        <button type="button" onClick={() => removeCommand(i)} className="p-1.5 text-gray-500 hover:text-red-400 hover:bg-gray-700 rounded">
                          <Trash2 className="w-3.5 h-3.5" />
                        </button>
                      )}
                    </div>
                  ))}
                </div>
                <button type="button" onClick={addCommand}
                  className="mt-2 flex items-center gap-1 text-xs text-gray-500 hover:text-gray-300">
                  <Plus className="w-3 h-3" /> Add command
                </button>
              </div>
              <div>
                <label className="block text-xs text-gray-400 mb-1">
                  Variables
                  <span className="ml-2 text-gray-600 font-normal">— JSON object; array value runs the command once per element</span>
                </label>
                <textarea
                  value={variablesJson}
                  onChange={e => { setVariablesJson(e.target.value); setJsonError(null) }}
                  rows={3}
                  spellCheck={false}
                  placeholder={'{\n  "repos": ["owner/repo-a", "owner/repo-b"]\n}'}
                  className="w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded text-sm font-mono text-white outline-none focus:border-blue-500 resize-none"
                />
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
