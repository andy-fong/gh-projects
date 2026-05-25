import { useState } from 'react'
import { X } from 'lucide-react'
import type { Note, CreateNoteInput } from '../../types'

interface Props {
  note?: Note
  initialValues?: Partial<CreateNoteInput>
  onConfirm: (input: CreateNoteInput) => void
  onClose: () => void
}

export default function NoteDialog({ note, initialValues, onConfirm, onClose }: Props) {
  const [title, setTitle] = useState(note?.title ?? initialValues?.title ?? '')
  const [body, setBody] = useState(note?.body ?? initialValues?.body ?? '')
  const [repo, setRepo] = useState(note?.repo ?? initialValues?.repo ?? '')
  const [refType, setRefType] = useState<string>(note?.ref_type ?? initialValues?.ref_type ?? '')
  const [refNumber, setRefNumber] = useState(
    note?.ref_number ? String(note.ref_number) :
    initialValues?.ref_number ? String(initialValues.ref_number) : ''
  )

  function handleSubmit(e: React.FormEvent) {
    e.preventDefault()
    onConfirm({
      title,
      body,
      repo: repo || undefined,
      ref_type: (refType as 'issue' | 'pr' | 'repo') || undefined,
      ref_number: refNumber ? Number(refNumber) : undefined,
    })
  }

  return (
    <div className="fixed inset-0 bg-black/60 flex items-center justify-center z-50" onClick={onClose}>
      <div className="bg-gray-800 border border-gray-700 rounded-lg p-6 w-[600px] shadow-xl" onClick={e => e.stopPropagation()}>
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-base font-semibold">{note ? 'Edit Note' : 'New Note'}</h2>
          <button onClick={onClose} className="text-gray-400 hover:text-white"><X className="w-4 h-4" /></button>
        </div>
        <form onSubmit={handleSubmit} className="space-y-3">
          <div>
            <label className="block text-xs text-gray-400 mb-1">Title</label>
            <input value={title} onChange={e => setTitle(e.target.value)} required className="w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded text-sm text-white outline-none focus:border-blue-500" />
          </div>
          <div className="grid grid-cols-3 gap-3">
            <div>
              <label className="block text-xs text-gray-400 mb-1">Repo (owner/repo)</label>
              <input value={repo} onChange={e => setRepo(e.target.value)} placeholder="org/repo" className="w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded text-sm text-white outline-none focus:border-blue-500" />
            </div>
            <div>
              <label className="block text-xs text-gray-400 mb-1">Ref type</label>
              <select value={refType} onChange={e => setRefType(e.target.value)} className="w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded text-sm text-white outline-none focus:border-blue-500">
                <option value="">None</option>
                <option value="issue">Issue</option>
                <option value="pr">PR</option>
                <option value="repo">Repo</option>
              </select>
            </div>
            <div>
              <label className="block text-xs text-gray-400 mb-1">Issue/PR #</label>
              <input type="number" value={refNumber} onChange={e => setRefNumber(e.target.value)} placeholder="123" className="w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded text-sm text-white outline-none focus:border-blue-500" />
            </div>
          </div>
          <div>
            <label className="block text-xs text-gray-400 mb-1">Body (Markdown)</label>
            <textarea
              value={body}
              onChange={e => setBody(e.target.value)}
              rows={10}
              className="w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded text-sm font-mono text-white outline-none focus:border-blue-500 resize-none"
              placeholder="Write your private note here…"
            />
          </div>
          <div className="flex justify-end gap-2 pt-2">
            <button type="button" onClick={onClose} className="px-4 py-2 text-sm text-gray-300 hover:text-white">Cancel</button>
            <button type="submit" className="px-4 py-2 text-sm bg-blue-600 hover:bg-blue-500 rounded text-white">{note ? 'Save' : 'Create'}</button>
          </div>
        </form>
      </div>
    </div>
  )
}
