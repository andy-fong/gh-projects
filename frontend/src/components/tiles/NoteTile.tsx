import { useQuery } from '@tanstack/react-query'
import ReactMarkdown from 'react-markdown'
import { api } from '../../api/client'
import type { NoteConfig } from '../../types'

interface Props {
  config: NoteConfig
}

export default function NoteTile({ config }: Props) {
  const { data: note, isLoading, error } = useQuery({
    queryKey: ['note', config.note_id],
    queryFn: () => api.notes.get(config.note_id),
  })

  if (isLoading) return <div className="text-gray-500 text-sm">Loading…</div>
  if (error || !note) return <div className="text-red-400 text-sm">Note not found</div>

  return (
    <div className="prose prose-invert prose-sm max-w-none">
      <ReactMarkdown>{note.body || '*No content*'}</ReactMarkdown>
    </div>
  )
}
