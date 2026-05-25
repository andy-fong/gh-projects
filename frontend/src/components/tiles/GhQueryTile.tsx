import { useState, useMemo, useEffect, useRef } from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { ChevronUp, ChevronDown, ChevronsUpDown, Columns, StickyNote } from 'lucide-react'
import { api } from '../../api/client'
import { extractDisplay } from '../../lib/fieldExtractors'
import { useSettings } from '../../context/SettingsContext'
import NoteDialog from '../dialogs/NoteDialog'
import type { CreateNoteInput, GhQueryConfig } from '../../types'

interface Props {
  config: GhQueryConfig
  tileId: number
}

type SortDir = 'asc' | 'desc'

function storageKey(tileId: number) {
  return `gh-tile-hidden-${tileId}`
}

function rowRefType(row: Record<string, unknown>, command: string): 'issue' | 'pr' {
  if ('isPullRequest' in row) return row.isPullRequest ? 'pr' : 'issue'
  const cmd = command.toLowerCase()
  if (cmd.startsWith('pr ') || cmd.includes(' prs ') || cmd.startsWith('prs ')) return 'pr'
  return 'issue'
}

function rowRepo(row: Record<string, unknown>): string | undefined {
  const repo = row.repository
  if (!repo) return undefined
  if (typeof repo === 'string') return repo
  if (typeof repo === 'object' && repo !== null) {
    const r = repo as Record<string, unknown>
    return (r.nameWithOwner ?? r.name) as string | undefined
  }
  return undefined
}

export default function GhQueryTile({ config, tileId }: Props) {
  const [sortKey, setSortKey] = useState<string | null>(null)
  const [sortDir, setSortDir] = useState<SortDir>('asc')
  const [hiddenCols, setHiddenCols] = useState<Set<string>>(() => {
    try {
      const stored = localStorage.getItem(storageKey(tileId))
      return stored ? new Set(JSON.parse(stored) as string[]) : new Set(['url'])
    } catch {
      return new Set(['url'])
    }
  })
  const [showColPicker, setShowColPicker] = useState(false)
  const [hoveredRow, setHoveredRow] = useState<number | null>(null)
  const [noteInit, setNoteInit] = useState<Partial<CreateNoteInput> | null>(null)
  const colPickerRef = useRef<HTMLDivElement>(null)
  const { globalExtractors } = useSettings()
  const qc = useQueryClient()

  useEffect(() => {
    localStorage.setItem(storageKey(tileId), JSON.stringify([...hiddenCols]))
  }, [hiddenCols, tileId])

  useEffect(() => {
    function handleClick(e: MouseEvent) {
      if (colPickerRef.current && !colPickerRef.current.contains(e.target as Node)) {
        setShowColPicker(false)
      }
    }
    document.addEventListener('mousedown', handleClick)
    return () => document.removeEventListener('mousedown', handleClick)
  }, [])

  const { data, isLoading, error } = useQuery({
    queryKey: ['gh', config.command],
    queryFn: () => api.gh.execute(config.command),
    staleTime: 60_000,
    retry: false,
  })

  const raw = useMemo<Record<string, unknown>[]>(() => {
    if (!data) return []
    const arr = Array.isArray(data.output) ? data.output : [data.output]
    return arr as Record<string, unknown>[]
  }, [data])

  const allKeys = useMemo(() => {
    if (raw.length === 0) return []
    return config.columns ?? Object.keys(raw[0])
  }, [raw, config.columns])

  const keys = useMemo(() => allKeys.filter(k => !hiddenCols.has(k)), [allKeys, hiddenCols])

  // Merge order: code defaults → global settings → per-tile override
  const extractors = useMemo(
    () => ({ ...globalExtractors, ...config.field_extractors }),
    [globalExtractors, config.field_extractors]
  )

  const rows = useMemo(() => {
    if (!sortKey) return raw
    return [...raw].sort((a, b) => {
      const av = extractDisplay(a[sortKey], extractors[sortKey])
      const bv = extractDisplay(b[sortKey], extractors[sortKey])
      const cmp = av.localeCompare(bv, undefined, { numeric: true, sensitivity: 'base' })
      return sortDir === 'asc' ? cmp : -cmp
    })
  }, [raw, sortKey, sortDir, extractors])

  if (isLoading) return <div className="text-gray-500 text-sm">Running gh command…</div>
  if (error) return <div className="text-red-400 text-sm">{(error as Error).message}</div>
  if (!data) return null
  if (raw.length === 0) return <div className="text-gray-500 text-sm">No results</div>

  function handleSort(key: string) {
    if (sortKey === key) {
      setSortDir(d => d === 'asc' ? 'desc' : 'asc')
    } else {
      setSortKey(key)
      setSortDir('asc')
    }
  }

  function toggleColumn(key: string) {
    setHiddenCols(prev => {
      const next = new Set(prev)
      if (next.has(key)) next.delete(key)
      else next.add(key)
      return next
    })
  }

  function openNoteForRow(row: Record<string, unknown>) {
    const repo = rowRepo(row)
    const ref_type = rowRefType(row, config.command)
    const ref_number = row.number !== undefined ? Number(row.number) : undefined
    const title = row.title ? String(row.title) : ''
    setNoteInit({ repo, ref_type, ref_number, title })
  }

  async function handleCreateNote(input: CreateNoteInput) {
    await api.notes.create(input)
    qc.invalidateQueries({ queryKey: ['notes'] })
    setNoteInit(null)
  }

  return (
    <div className="h-full flex flex-col gap-1">
      <div className="flex justify-end shrink-0 relative" ref={colPickerRef}>
        <button
          onClick={() => setShowColPicker(v => !v)}
          className={`flex items-center gap-1 px-2 py-0.5 text-xs rounded ${showColPicker ? 'bg-gray-700 text-gray-200' : 'text-gray-500 hover:text-gray-200 hover:bg-gray-700'}`}
        >
          <Columns className="w-3 h-3" /> Columns
        </button>
        {showColPicker && (
          <div className="absolute top-full right-0 mt-1 bg-gray-900 border border-gray-700 rounded shadow-xl z-10 py-1 min-w-36">
            {allKeys.map(k => (
              <label key={k} className="flex items-center gap-2 px-3 py-1.5 hover:bg-gray-700 cursor-pointer text-xs text-gray-300">
                <input
                  type="checkbox"
                  checked={!hiddenCols.has(k)}
                  onChange={() => toggleColumn(k)}
                  className="accent-blue-500"
                />
                {k}
              </label>
            ))}
          </div>
        )}
      </div>

      <div className="overflow-auto flex-1">
        <table className="w-full text-xs">
          <thead>
            <tr className="border-b border-gray-700">
              {keys.map(k => (
                <th
                  key={k}
                  onClick={() => handleSort(k)}
                  className="text-left py-1 px-2 text-gray-400 font-medium capitalize cursor-pointer select-none hover:text-gray-200"
                >
                  <span className="inline-flex items-center gap-1">
                    {k}
                    {sortKey === k
                      ? sortDir === 'asc'
                        ? <ChevronUp className="w-3 h-3" />
                        : <ChevronDown className="w-3 h-3" />
                      : <ChevronsUpDown className="w-3 h-3 opacity-30" />}
                  </span>
                </th>
              ))}
              <th className="w-6" />
            </tr>
          </thead>
          <tbody>
            {rows.map((row, i) => (
              <tr
                key={i}
                className="border-b border-gray-700/50 hover:bg-gray-700/30"
                onMouseEnter={() => setHoveredRow(i)}
                onMouseLeave={() => setHoveredRow(null)}
              >
                {keys.map(k => {
                  const val = row[k]
                  const url = row['url'] as string | undefined
                  const display = extractDisplay(val, extractors[k])
                  const cell = k === 'number' && url
                    ? <a href={url} target="_blank" rel="noopener noreferrer" className="text-blue-400 hover:text-blue-300 hover:underline">{display}</a>
                    : display
                  return <td key={k} className="py-1 px-2 text-gray-300 max-w-xs truncate">{cell}</td>
                })}
                <td className="py-1 px-1 w-6">
                  {hoveredRow === i && (
                    <button
                      onClick={() => openNoteForRow(row)}
                      title="Add private note"
                      className="p-0.5 rounded text-gray-500 hover:text-yellow-400 hover:bg-gray-600"
                    >
                      <StickyNote className="w-3.5 h-3.5" />
                    </button>
                  )}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {noteInit !== null && (
        <NoteDialog
          initialValues={noteInit}
          onConfirm={handleCreateNote}
          onClose={() => setNoteInit(null)}
        />
      )}
    </div>
  )
}
