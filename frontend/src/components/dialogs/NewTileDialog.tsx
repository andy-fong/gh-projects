import { useState } from 'react'
import { X, Plus, Trash2 } from 'lucide-react'
import type { CreateTileInput } from '../../types'

export interface NewTileInitialValues {
  type: 'gh_query' | 'note'
  title: string
  commands?: string[]
  variablesJson?: string
  noteId?: string
}

interface Props {
  onConfirm: (input: CreateTileInput) => void
  onClose: () => void
  initialValues?: NewTileInitialValues
}

export default function NewTileDialog({ onConfirm, onClose, initialValues }: Props) {
  const [type, setType] = useState<'gh_query' | 'note'>(initialValues?.type ?? 'gh_query')
  const [title, setTitle] = useState(initialValues?.title ?? '')
  const [commands, setCommands] = useState(initialValues?.commands ?? [''])
  const [noteId, setNoteId] = useState(initialValues?.noteId ?? '')
  const [variablesJson, setVariablesJson] = useState(initialValues?.variablesJson ?? '')
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
    if (type === 'gh_query') {
      const filled = commands.filter(c => c.trim())
      if (filled.length === 0) return
      try {
        const variables = variablesJson.trim() ? JSON.parse(variablesJson) : {}
        if (typeof variables !== 'object' || Array.isArray(variables))
          throw new Error('Must be a JSON object')
        onConfirm({ title: title || 'GH Query', tile_type: type, config: { command: filled[0], commands: filled, variables } })
      } catch (err) {
        setJsonError((err as Error).message)
      }
    } else {
      onConfirm({ title: title || 'Note', tile_type: type, config: { note_id: Number(noteId) } })
    }
  }

  return (
    <div className="fixed inset-0 bg-black/60 flex items-center justify-center z-50" onClick={onClose}>
      <div className="bg-gray-800 border border-gray-700 rounded-lg p-6 w-[560px] shadow-xl" onClick={e => e.stopPropagation()}>
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-base font-semibold">{initialValues ? 'Paste Tile' : 'Add Tile'}</h2>
          <button onClick={onClose} className="text-gray-400 hover:text-white"><X className="w-4 h-4" /></button>
        </div>
        <form onSubmit={handleSubmit} className="space-y-4">
          <div>
            <label className="block text-xs text-gray-400 mb-1">Tile type</label>
            <div className="flex gap-2">
              {(['gh_query', 'note'] as const).map(t => (
                <button key={t} type="button" onClick={() => setType(t)}
                  className={`px-3 py-1.5 rounded text-sm border ${type === t ? 'bg-blue-600 border-blue-500 text-white' : 'border-gray-600 text-gray-300 hover:border-gray-500'}`}>
                  {t === 'gh_query' ? 'GH Query' : 'Note'}
                </button>
              ))}
            </div>
          </div>
          <div>
            <label className="block text-xs text-gray-400 mb-1">Title</label>
            <input value={title} onChange={e => setTitle(e.target.value)}
              placeholder={type === 'gh_query' ? 'Open Issues' : 'My Note'}
              className="w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded text-sm text-white outline-none focus:border-blue-500" />
          </div>
          {type === 'gh_query' ? (
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
                      placeholder="issue list --repo owner/repo --milestone 2.3 --state all --json number,title,state,assignees,updatedAt,url"
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
              <p className="text-xs text-gray-600 mt-1">Omit the leading "gh ". Use <code className="text-gray-500">{'{{var}}'}</code> for template variables. Click any cell value to filter by it.</p>
            </div>
          ) : (
            <div>
              <label className="block text-xs text-gray-400 mb-1">Note ID</label>
              <input type="number" value={noteId} onChange={e => setNoteId(e.target.value)} placeholder="1"
                className="w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded text-sm text-white outline-none focus:border-blue-500" required />
            </div>
          )}
          {type === 'gh_query' && (
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
              {jsonError && <p className="text-xs text-red-400 mt-1">{jsonError}</p>}
            </div>
          )}
          <div className="flex justify-end gap-2 pt-2">
            <button type="button" onClick={onClose} className="px-4 py-2 text-sm text-gray-300 hover:text-white">Cancel</button>
            <button type="submit" className="px-4 py-2 text-sm bg-blue-600 hover:bg-blue-500 rounded text-white">Add tile</button>
          </div>
        </form>
      </div>
    </div>
  )
}
