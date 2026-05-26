import { useState } from 'react'
import { createPortal } from 'react-dom'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import ReactMarkdown from 'react-markdown'
import { X, ExternalLink, Plus, GitPullRequest, CircleDot, GitMerge, MessageSquare, StickyNote } from 'lucide-react'
import { api } from '../api/client'
import NoteDialog from './dialogs/NoteDialog'
import type { CreateNoteInput } from '../types'

interface Props {
  repo: string
  refType: 'issue' | 'pr'
  number: number
  url: string
  onClose: () => void
}

interface GhItem {
  number: number
  title: string
  state: string
  body: string
  author: { login: string; name?: string }
  assignees: Array<{ login: string; name?: string }>
  labels: Array<{ name: string; color?: string }>
  milestone: { title: string } | null
  comments: Array<{ author: { login: string }; body: string; createdAt: string }>
  url: string
  createdAt: string
  updatedAt: string
  isDraft?: boolean
  headRefName?: string
  baseRefName?: string
  reviewDecision?: string
}

function StateBadge({ item, refType }: { item: GhItem; refType: 'issue' | 'pr' }) {
  const isDraft = refType === 'pr' && item.isDraft
  const state = isDraft ? 'draft' : item.state.toLowerCase()

  const styles: Record<string, string> = {
    open:   'bg-green-700 text-green-100',
    closed: refType === 'pr' ? 'bg-red-700 text-red-100' : 'bg-purple-700 text-purple-100',
    merged: 'bg-purple-700 text-purple-100',
    draft:  'bg-gray-600 text-gray-200',
  }

  const icons: Record<string, React.ReactNode> = {
    open:   refType === 'pr' ? <GitPullRequest className="w-3 h-3" /> : <CircleDot className="w-3 h-3" />,
    closed: <X className="w-3 h-3" />,
    merged: <GitMerge className="w-3 h-3" />,
    draft:  <GitPullRequest className="w-3 h-3" />,
  }

  return (
    <span className={`inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs font-medium ${styles[state] ?? styles.open}`}>
      {icons[state] ?? icons.open}
      {state}
    </span>
  )
}

function Avatar({ login }: { login: string }) {
  return (
    <img
      src={`https://github.com/${login}.png?size=20`}
      alt={login}
      className="w-5 h-5 rounded-full inline-block"
    />
  )
}

function Comment({ author, body, createdAt, label }: { author: string; body: string; createdAt: string; label?: string }) {
  return (
    <div className="border border-gray-700 rounded-lg overflow-hidden">
      <div className="flex items-center gap-2 px-3 py-2 bg-gray-700/40 border-b border-gray-700">
        <Avatar login={author} />
        <span className="text-xs text-gray-300 font-medium">{author}</span>
        <span className="text-xs text-gray-600 ml-auto">{new Date(createdAt).toLocaleDateString()}</span>
        {label && <span className="text-xs text-gray-500 italic">{label}</span>}
      </div>
      <div className="px-3 py-2 prose prose-invert prose-xs max-w-none text-gray-300 text-xs">
        <ReactMarkdown>{body || '*No content*'}</ReactMarkdown>
      </div>
    </div>
  )
}

export default function DetailPanel({ repo, refType, number, url, onClose }: Props) {
  const qc = useQueryClient()
  const [showNoteDialog, setShowNoteDialog] = useState(false)

  const jsonFields = refType === 'pr'
    ? 'number,title,state,body,author,assignees,labels,milestone,comments,url,createdAt,updatedAt,isDraft,headRefName,baseRefName,reviewDecision'
    : 'number,title,state,body,author,assignees,labels,milestone,comments,url,createdAt,updatedAt'

  const cmd = `${refType} view ${number} --repo ${repo} --json ${jsonFields}`

  const { data: ghData, isLoading, error } = useQuery({
    queryKey: ['gh', cmd],
    queryFn: () => api.gh.execute(cmd),
    staleTime: 60_000,
  })

  const { data: notes = [] } = useQuery({
    queryKey: ['notes', repo, refType, number],
    queryFn: () => api.notes.list({ repo, ref_type: refType, ref_number: number }),
  })

  async function handleCreateNote(input: CreateNoteInput) {
    await api.notes.create(input)
    qc.invalidateQueries({ queryKey: ['notes', repo, refType, number] })
    qc.invalidateQueries({ queryKey: ['notes'] })
    setShowNoteDialog(false)
  }

  const item = ghData?.output as GhItem | undefined
  const lastComment = item?.comments?.length ? item.comments[item.comments.length - 1] : null

  const panel = (
    <>
      {/* Backdrop */}
      <div
        className="fixed inset-0 z-40 bg-black/30"
        onClick={onClose}
      />

      {/* Panel */}
      <div className="fixed top-0 right-0 h-full w-[480px] z-50 bg-gray-900 border-l border-gray-700 flex flex-col shadow-2xl">
        {/* Header */}
        <div className="flex items-start gap-2 px-4 py-3 border-b border-gray-700 shrink-0">
          <div className="flex-1 min-w-0">
            {isLoading && <div className="text-sm text-gray-500">Loading…</div>}
            {error && <div className="text-sm text-red-400">{(error as Error).message}</div>}
            {item && (
              <>
                <div className="flex items-center gap-2 mb-1">
                  <StateBadge item={item} refType={refType} />
                  <span className="text-xs text-gray-500">#{item.number}</span>
                  {item.milestone && (
                    <span className="text-xs text-gray-500 bg-gray-700 px-1.5 py-0.5 rounded">
                      {item.milestone.title}
                    </span>
                  )}
                </div>
                <h2 className="text-sm font-semibold text-white leading-snug">{item.title}</h2>
                <div className="flex items-center gap-3 mt-1.5 flex-wrap">
                  <span className="flex items-center gap-1 text-xs text-gray-500">
                    <Avatar login={item.author.login} />
                    {item.author.name || item.author.login}
                  </span>
                  {item.assignees.length > 0 && (
                    <span className="flex items-center gap-1 text-xs text-gray-500">
                      → {item.assignees.map(a => (
                        <span key={a.login} className="flex items-center gap-1">
                          <Avatar login={a.login} />
                          {a.name || a.login}
                        </span>
                      ))}
                    </span>
                  )}
                </div>
                {item.labels.length > 0 && (
                  <div className="flex gap-1 mt-1.5 flex-wrap">
                    {item.labels.map(l => (
                      <span key={l.name} className="px-1.5 py-0.5 rounded-full text-xs bg-gray-700 text-gray-300">
                        {l.name}
                      </span>
                    ))}
                  </div>
                )}
                {refType === 'pr' && item.headRefName && (
                  <div className="text-xs text-gray-600 mt-1">
                    <code>{item.headRefName}</code> → <code>{item.baseRefName}</code>
                  </div>
                )}
              </>
            )}
          </div>
          <div className="flex items-center gap-1 shrink-0">
            <a href={url} target="_blank" rel="noopener noreferrer" className="p-1.5 rounded text-gray-500 hover:text-gray-200 hover:bg-gray-700" title="Open on GitHub">
              <ExternalLink className="w-4 h-4" />
            </a>
            <button onClick={onClose} className="p-1.5 rounded text-gray-500 hover:text-gray-200 hover:bg-gray-700">
              <X className="w-4 h-4" />
            </button>
          </div>
        </div>

        {/* Scrollable body */}
        <div className="flex-1 overflow-y-auto px-4 py-3 space-y-4">
          {item && (
            <>
              {/* Description */}
              <section>
                <Comment
                  author={item.author.login}
                  body={item.body}
                  createdAt={item.createdAt}
                  label="description"
                />
              </section>

              {/* Last comment */}
              {lastComment && (
                <section>
                  <div className="flex items-center gap-2 mb-2 text-xs text-gray-500">
                    <MessageSquare className="w-3.5 h-3.5" />
                    Last comment
                    {item.comments.length > 1 && (
                      <a href={url} target="_blank" rel="noopener noreferrer" className="ml-auto text-blue-500 hover:text-blue-400">
                        {item.comments.length} total ↗
                      </a>
                    )}
                  </div>
                  <Comment
                    author={lastComment.author.login}
                    body={lastComment.body}
                    createdAt={lastComment.createdAt}
                  />
                </section>
              )}

              {/* Private notes */}
              <section>
                <div className="flex items-center gap-2 mb-2 text-xs text-gray-500">
                  <StickyNote className="w-3.5 h-3.5" />
                  Private notes ({notes.length})
                  <button
                    onClick={() => setShowNoteDialog(true)}
                    className="ml-auto flex items-center gap-1 text-blue-500 hover:text-blue-400"
                  >
                    <Plus className="w-3 h-3" /> Add note
                  </button>
                </div>
                {notes.length === 0 ? (
                  <div className="text-xs text-gray-600 italic">No notes yet</div>
                ) : (
                  <div className="space-y-2">
                    {notes.map(note => (
                      <div key={note.id} className="border border-gray-700 rounded-lg px-3 py-2">
                        <div className="text-xs font-medium text-gray-300 mb-1">{note.title}</div>
                        <div className="prose prose-invert prose-xs max-w-none text-gray-400 text-xs">
                          <ReactMarkdown>{note.body || '*No content*'}</ReactMarkdown>
                        </div>
                      </div>
                    ))}
                  </div>
                )}
              </section>
            </>
          )}
        </div>
      </div>

      {showNoteDialog && (
        <NoteDialog
          initialValues={{ repo, ref_type: refType, ref_number: number }}
          onConfirm={handleCreateNote}
          onClose={() => setShowNoteDialog(false)}
        />
      )}
    </>
  )

  return createPortal(panel, document.body)
}
