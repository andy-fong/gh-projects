import { useState } from 'react'
import { X, Plus, Trash2 } from 'lucide-react'
import type { CreateTileInput } from '../../types'

interface Props {
  onConfirm: (input: CreateTileInput) => void
  onClose: () => void
}

export default function NewTileDialog({ onConfirm, onClose }: Props) {
  const [type, setType] = useState<'gh_query' | 'note'>('gh_query')
  const [title, setTitle] = useState('')
  const [commands, setCommands] = useState([''])
  const [noteId, setNoteId] = useState('')

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
    const filled = commands.filter(c => c.trim())
    if (type === 'gh_query' && filled.length === 0) return
    const config = type === 'gh_query'
      ? { command: filled[0], commands: filled }
      : { note_id: Number(noteId) }
    onConfirm({ title: title || (type === 'gh_query' ? 'GH Query' : 'Note'), tile_type: type, config })
  }

  return (
    <div className="fixed inset-0 bg-black/60 flex items-center justify-center z-50" onClick={onClose}>
      <div className="bg-gray-800 border border-gray-700 rounded-lg p-6 w-[560px] shadow-xl" onClick={e => e.stopPropagation()}>
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-base font-semibold">Add Tile</h2>
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
              <p className="text-xs text-gray-600 mt-1">Omit the leading "gh ". Click any cell value to filter by it.</p>
            </div>
          ) : (
            <div>
              <label className="block text-xs text-gray-400 mb-1">Note ID</label>
              <input type="number" value={noteId} onChange={e => setNoteId(e.target.value)} placeholder="1"
                className="w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded text-sm text-white outline-none focus:border-blue-500" required />
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
