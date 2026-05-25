import { useState } from 'react'
import { X } from 'lucide-react'
import type { CreateTileInput } from '../../types'

interface Props {
  onConfirm: (input: CreateTileInput) => void
  onClose: () => void
}

export default function NewTileDialog({ onConfirm, onClose }: Props) {
  const [type, setType] = useState<'gh_query' | 'note'>('gh_query')
  const [title, setTitle] = useState('')
  const [command, setCommand] = useState('issue list --json number,title,state,assignees --limit 20')
  const [noteId, setNoteId] = useState('')

  function handleSubmit(e: React.FormEvent) {
    e.preventDefault()
    const config = type === 'gh_query'
      ? { command }
      : { note_id: Number(noteId) }
    onConfirm({ title: title || (type === 'gh_query' ? 'GH Query' : 'Note'), tile_type: type, config })
  }

  return (
    <div className="fixed inset-0 bg-black/60 flex items-center justify-center z-50" onClick={onClose}>
      <div className="bg-gray-800 border border-gray-700 rounded-lg p-6 w-[500px] shadow-xl" onClick={e => e.stopPropagation()}>
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-base font-semibold">Add Tile</h2>
          <button onClick={onClose} className="text-gray-400 hover:text-white"><X className="w-4 h-4" /></button>
        </div>
        <form onSubmit={handleSubmit} className="space-y-4">
          <div>
            <label className="block text-xs text-gray-400 mb-1">Tile type</label>
            <div className="flex gap-2">
              {(['gh_query', 'note'] as const).map(t => (
                <button
                  key={t}
                  type="button"
                  onClick={() => setType(t)}
                  className={`px-3 py-1.5 rounded text-sm border ${type === t ? 'bg-blue-600 border-blue-500 text-white' : 'border-gray-600 text-gray-300 hover:border-gray-500'}`}
                >
                  {t === 'gh_query' ? 'GH Query' : 'Note'}
                </button>
              ))}
            </div>
          </div>
          <div>
            <label className="block text-xs text-gray-400 mb-1">Title</label>
            <input value={title} onChange={e => setTitle(e.target.value)} placeholder={type === 'gh_query' ? 'Open Issues' : 'My Note'} className="w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded text-sm text-white outline-none focus:border-blue-500" />
          </div>
          {type === 'gh_query' ? (
            <div>
              <label className="block text-xs text-gray-400 mb-1">gh command (without "gh ")</label>
              <input value={command} onChange={e => setCommand(e.target.value)} className="w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded text-sm font-mono text-white outline-none focus:border-blue-500" />
              <p className="text-xs text-gray-500 mt-1">Example: issue list --repo owner/repo --json number,title,state</p>
            </div>
          ) : (
            <div>
              <label className="block text-xs text-gray-400 mb-1">Note ID</label>
              <input type="number" value={noteId} onChange={e => setNoteId(e.target.value)} placeholder="1" className="w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded text-sm text-white outline-none focus:border-blue-500" required />
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
