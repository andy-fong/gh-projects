import { useState } from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import ReactMarkdown from 'react-markdown'
import { Plus, Trash2, Edit2, ExternalLink } from 'lucide-react'
import { api } from '../api/client'
import NoteDialog from '../components/dialogs/NoteDialog'
import type { CreateNoteInput, Note } from '../types'

export default function NotesPage() {
  const qc = useQueryClient()
  const { data: notes = [] } = useQuery({ queryKey: ['notes'], queryFn: () => api.notes.list() })
  const [selected, setSelected] = useState<Note | null>(null)
  const [showCreate, setShowCreate] = useState(false)
  const [editing, setEditing] = useState<Note | null>(null)

  async function handleCreate(input: CreateNoteInput) {
    await api.notes.create(input)
    qc.invalidateQueries({ queryKey: ['notes'] })
    setShowCreate(false)
  }

  async function handleUpdate(input: CreateNoteInput) {
    if (!editing) return
    await api.notes.update(editing.id, input)
    qc.invalidateQueries({ queryKey: ['notes'] })
    qc.invalidateQueries({ queryKey: ['note', editing.id] })
    setEditing(null)
    setSelected(null)
  }

  async function handleDelete(note: Note) {
    if (!confirm(`Delete "${note.title}"?`)) return
    await api.notes.delete(note.id)
    qc.invalidateQueries({ queryKey: ['notes'] })
    if (selected?.id === note.id) setSelected(null)
  }

  return (
    <div className="flex h-full">
      <div className="w-72 border-r border-gray-700 flex flex-col">
        <div className="px-4 py-3 border-b border-gray-700 flex items-center justify-between">
          <h2 className="text-sm font-semibold">Private Notes</h2>
          <button onClick={() => setShowCreate(true)} className="p-1 rounded hover:bg-gray-700 text-gray-400 hover:text-white">
            <Plus className="w-4 h-4" />
          </button>
        </div>
        <div className="flex-1 overflow-y-auto">
          {notes.map(note => (
            <button
              key={note.id}
              onClick={() => setSelected(note)}
              className={`w-full text-left px-4 py-3 border-b border-gray-700/50 hover:bg-gray-800 ${selected?.id === note.id ? 'bg-gray-800' : ''}`}
            >
              <div className="text-sm font-medium text-gray-200 truncate">{note.title}</div>
              {note.repo && (
                <div className="text-xs text-gray-500 mt-0.5">
                  {note.repo}{note.ref_number ? ` #${note.ref_number}` : ''}
                </div>
              )}
              <div className="text-xs text-gray-600 mt-0.5">{new Date(note.updated_at).toLocaleDateString()}</div>
            </button>
          ))}
          {notes.length === 0 && (
            <div className="px-4 py-8 text-sm text-gray-500 text-center">No notes yet</div>
          )}
        </div>
      </div>

      <div className="flex-1 flex flex-col overflow-hidden">
        {selected ? (
          <>
            <div className="px-6 py-3 border-b border-gray-700 flex items-center justify-between">
              <div>
                <h2 className="text-sm font-semibold">{selected.title}</h2>
                {selected.repo && (
                  <div className="text-xs text-gray-500 mt-0.5">
                    {selected.repo}{selected.ref_number ? (
                      <> · {selected.ref_type} <a
                        href={`https://github.com/${selected.repo}/${selected.ref_type === 'pr' ? 'pull' : 'issues'}/${selected.ref_number}`}
                        target="_blank"
                        rel="noopener noreferrer"
                        className="hover:text-gray-300 hover:underline"
                        onClick={e => e.stopPropagation()}
                      >#{selected.ref_number}</a></>
                    ) : ''}
                  </div>
                )}
              </div>
              <div className="flex gap-2">
                {selected.repo && selected.ref_number && (
                  <a
                    href={`https://github.com/${selected.repo}/${selected.ref_type === 'pr' ? 'pull' : 'issues'}/${selected.ref_number}`}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="p-1.5 rounded hover:bg-gray-700 text-gray-400 hover:text-white"
                    title="Open on GitHub"
                  >
                    <ExternalLink className="w-4 h-4" />
                  </a>
                )}
                <button onClick={() => setEditing(selected)} className="p-1.5 rounded hover:bg-gray-700 text-gray-400 hover:text-white">
                  <Edit2 className="w-4 h-4" />
                </button>
                <button onClick={() => handleDelete(selected)} className="p-1.5 rounded hover:bg-gray-700 text-gray-400 hover:text-red-400">
                  <Trash2 className="w-4 h-4" />
                </button>
              </div>
            </div>
            <div className="flex-1 overflow-auto p-6">
              <div className="prose prose-invert prose-sm max-w-none">
                <ReactMarkdown>{selected.body || '*No content*'}</ReactMarkdown>
              </div>
            </div>
          </>
        ) : (
          <div className="flex-1 flex items-center justify-center text-gray-500 text-sm">
            Select a note to read it
          </div>
        )}
      </div>

      {showCreate && <NoteDialog onConfirm={handleCreate} onClose={() => setShowCreate(false)} />}
      {editing && <NoteDialog note={editing} onConfirm={handleUpdate} onClose={() => setEditing(null)} />}
    </div>
  )
}
