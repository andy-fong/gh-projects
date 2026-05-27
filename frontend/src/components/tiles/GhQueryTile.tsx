import { useState, useMemo, useEffect, useRef } from 'react'
import { createPortal } from 'react-dom'
import { useQueries, useQueryClient } from '@tanstack/react-query'
import { ChevronUp, ChevronDown, ChevronsUpDown, Columns, StickyNote, Filter, X, Search } from 'lucide-react'
import { api } from '../../api/client'
import { extractDisplay } from '../../lib/fieldExtractors'
import { useSettings } from '../../context/SettingsContext'
import NoteDialog from '../dialogs/NoteDialog'
import DetailPanel from '../DetailPanel'
import Tooltip from '../Tooltip'
import type { CreateNoteInput, GhQueryConfig } from '../../types'

interface Props {
  config: GhQueryConfig
  tileId: number
}

type SortDir = 'asc' | 'desc'
type Filters = Record<string, string[]>

function storageKey(tileId: number) { return `gh-tile-hidden-${tileId}` }

function parseRepoFromCommand(cmd: string): string | undefined {
  return cmd.match(/--repo\s+(\S+)/)?.[1]
}

function rowRefType(row: Record<string, unknown>, commands: string[]): 'issue' | 'pr' {
  if ('isPullRequest' in row) return row.isPullRequest ? 'pr' : 'issue'
  const cmd = (commands[0] ?? '').toLowerCase()
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

interface FilterDropdownProps {
  col: string
  values: string[]
  selected: string[]
  anchor: DOMRect
  onToggle: (val: string) => void
  onSelectAll: () => void
  onClear: () => void
  onClose: () => void
}

function FilterDropdown({ col, values, selected, anchor, onToggle, onSelectAll, onClear, onClose }: FilterDropdownProps) {
  const [search, setSearch] = useState('')
  const ref = useRef<HTMLDivElement>(null)

  useEffect(() => {
    function handleClick(e: MouseEvent) {
      if (ref.current && !ref.current.contains(e.target as Node)) onClose()
    }
    document.addEventListener('mousedown', handleClick)
    return () => document.removeEventListener('mousedown', handleClick)
  }, [onClose])

  const filtered = values.filter(v => v.toLowerCase().includes(search.toLowerCase()))

  const style: React.CSSProperties = {
    position: 'fixed',
    top: anchor.bottom + 4,
    left: anchor.left,
    zIndex: 9999,
    minWidth: Math.max(anchor.width, 180),
    maxWidth: 280,
  }

  return createPortal(
    <div ref={ref} style={style} className="bg-gray-900 border border-gray-700 rounded shadow-xl flex flex-col max-h-64">
      <div className="flex items-center gap-1 px-2 pt-2 pb-1 shrink-0">
        <Search className="w-3 h-3 text-gray-500 shrink-0" />
        <input
          autoFocus
          value={search}
          onChange={e => setSearch(e.target.value)}
          placeholder={`Filter ${col}…`}
          className="flex-1 bg-transparent text-xs text-white outline-none placeholder-gray-600"
        />
      </div>
      <div className="flex gap-2 px-2 pb-1 shrink-0 border-b border-gray-700">
        <button onClick={onSelectAll} className="text-xs text-blue-400 hover:text-blue-300">All</button>
        <button onClick={onClear} className="text-xs text-gray-500 hover:text-gray-300">Clear</button>
        {selected.length > 0 && <span className="text-xs text-gray-600 ml-auto">{selected.length} selected</span>}
      </div>
      <div className="overflow-y-auto flex-1 py-1">
        {filtered.length === 0
          ? <div className="px-3 py-2 text-xs text-gray-600">No values</div>
          : filtered.map(val => (
            <label key={val} className="flex items-center gap-2 px-3 py-1 hover:bg-gray-700 cursor-pointer text-xs text-gray-300">
              <input
                type="checkbox"
                checked={selected.includes(val)}
                onChange={() => onToggle(val)}
                className="accent-blue-500 shrink-0"
              />
              <span className="truncate">{val || <span className="text-gray-600 italic">empty</span>}</span>
            </label>
          ))
        }
      </div>
    </div>,
    document.body
  )
}

export default function GhQueryTile({ config, tileId }: Props) {
  const [sortKey, setSortKey] = useState<string | null>(null)
  const [sortDir, setSortDir] = useState<SortDir>('asc')
  const [hiddenCols, setHiddenCols] = useState<Set<string>>(() => {
    try {
      const stored = localStorage.getItem(storageKey(tileId))
      return stored ? new Set(JSON.parse(stored) as string[]) : new Set(['url'])
    } catch { return new Set(['url']) }
  })
  const [showColPicker, setShowColPicker] = useState(false)
  const [showFilters, setShowFilters] = useState(false)
  const [filters, setFilters] = useState<Filters>({})
  const [openFilterCol, setOpenFilterCol] = useState<string | null>(null)
  const [filterAnchor, setFilterAnchor] = useState<DOMRect | null>(null)
  const [hoveredRow, setHoveredRow] = useState<number | null>(null)
  const [noteInit, setNoteInit] = useState<Partial<CreateNoteInput> | null>(null)
  const [detailItem, setDetailItem] = useState<{ repo: string; refType: 'issue' | 'pr'; number: number; url: string } | null>(null)
  const colPickerRef = useRef<HTMLDivElement>(null)
  const { globalExtractors } = useSettings()
  const qc = useQueryClient()

  const commands = useMemo(() => {
    const base = config.commands?.length ? config.commands : [config.command]
    const vars = config.variables ?? {}
    const arrayEntry = Object.entries(vars).find(([, v]) => Array.isArray(v)) as [string, string[]] | undefined
    return base.flatMap(cmd => {
      let expanded = cmd
      for (const [k, v] of Object.entries(vars)) {
        if (!Array.isArray(v)) expanded = expanded.replaceAll(`{{${k}}}`, v)
      }
      if (arrayEntry) {
        const [key, values] = arrayEntry
        return values.map(val => expanded.replaceAll(`{{${key}}}`, val))
      }
      return [expanded]
    })
  }, [config.commands, config.command, config.variables])

  useEffect(() => {
    localStorage.setItem(storageKey(tileId), JSON.stringify([...hiddenCols]))
  }, [hiddenCols, tileId])

  useEffect(() => {
    function handleClick(e: MouseEvent) {
      if (colPickerRef.current && !colPickerRef.current.contains(e.target as Node))
        setShowColPicker(false)
    }
    document.addEventListener('mousedown', handleClick)
    return () => document.removeEventListener('mousedown', handleClick)
  }, [])

  const queryResults = useQueries({
    queries: commands.map(cmd => ({
      queryKey: ['gh', cmd],
      queryFn: () => api.gh.execute(cmd),
      staleTime: 60_000,
      retry: false,
    })),
  })

  const isLoading = queryResults.some(r => r.isLoading)
  const errors = queryResults.filter(r => r.error).map(r => (r.error as Error).message)

  const raw = useMemo<Record<string, unknown>[]>(() => {
    return queryResults.flatMap((r, i) => {
      if (!r.data) return []
      const arr = Array.isArray(r.data.output) ? r.data.output : [r.data.output]
      const repoFromCmd = parseRepoFromCommand(commands[i] ?? '')
      return (arr as Record<string, unknown>[]).map(row => {
        if (repoFromCmd && !row.repository) {
          const name = repoFromCmd.split('/')[1] ?? repoFromCmd
          return { ...row, repository: { nameWithOwner: repoFromCmd, name } }
        }
        return row
      })
    })
  }, [queryResults, commands])

  const allKeys = useMemo(() => {
    if (raw.length === 0) return []
    const seen = new Set<string>()
    for (const row of raw) for (const k of Object.keys(row)) seen.add(k)
    return config.columns ?? [...seen]
  }, [raw, config.columns])

  const keys = useMemo(() => allKeys.filter(k => !hiddenCols.has(k)), [allKeys, hiddenCols])

  const extractors = useMemo(
    () => ({ ...globalExtractors, ...config.field_extractors }),
    [globalExtractors, config.field_extractors]
  )

  const sorted = useMemo(() => {
    if (!sortKey) return raw
    return [...raw].sort((a, b) => {
      const av = extractDisplay(a[sortKey], extractors[sortKey])
      const bv = extractDisplay(b[sortKey], extractors[sortKey])
      const cmp = av.localeCompare(bv, undefined, { numeric: true, sensitivity: 'base' })
      return sortDir === 'asc' ? cmp : -cmp
    })
  }, [raw, sortKey, sortDir, extractors])

  // Unique values per column for the filter dropdowns (computed from unfiltered data)
  const columnValues = useMemo(() => {
    const map: Record<string, string[]> = {}
    for (const k of allKeys) {
      const seen = new Set<string>()
      for (const row of sorted) seen.add(extractDisplay(row[k], extractors[k]))
      map[k] = [...seen].sort((a, b) => a.localeCompare(b, undefined, { numeric: true }))
    }
    return map
  }, [sorted, allKeys, extractors])

  const activeFilterCount = Object.values(filters).filter(v => v.length > 0).length

  const rows = useMemo(() => {
    if (activeFilterCount === 0) return sorted
    return sorted.filter(row =>
      Object.entries(filters).every(([col, vals]) => {
        if (!vals.length) return true
        return vals.includes(extractDisplay(row[col], extractors[col]))
      })
    )
  }, [sorted, filters, extractors, activeFilterCount])

  if (isLoading) return <div className="text-gray-500 text-sm">Running gh command…</div>
  if (errors.length > 0 && raw.length === 0) return (
    <div className="space-y-1">{errors.map((e, i) => <div key={i} className="text-red-400 text-sm">{e}</div>)}</div>
  )
  if (raw.length === 0) return <div className="text-gray-500 text-sm">No results</div>

  function handleSort(key: string) {
    if (sortKey === key) setSortDir(d => d === 'asc' ? 'desc' : 'asc')
    else { setSortKey(key); setSortDir('asc') }
  }

  function toggleColumn(key: string) {
    setHiddenCols(prev => {
      const next = new Set(prev)
      if (next.has(key)) next.delete(key); else next.add(key)
      return next
    })
  }

  function openFilter(col: string, e: React.MouseEvent) {
    e.stopPropagation()
    if (openFilterCol === col) { setOpenFilterCol(null); return }
    setFilterAnchor((e.currentTarget as HTMLElement).getBoundingClientRect())
    setOpenFilterCol(col)
    setShowFilters(true)
  }

  function toggleFilterValue(col: string, val: string) {
    setFilters(prev => {
      const cur = prev[col] ?? []
      return { ...prev, [col]: cur.includes(val) ? cur.filter(v => v !== val) : [...cur, val] }
    })
  }

  function handleCellClick(col: string, display: string) {
    if (!display) return
    setFilters(prev => {
      const cur = prev[col] ?? []
      return { ...prev, [col]: cur.includes(display) ? cur.filter(v => v !== display) : [...cur, display] }
    })
    setShowFilters(true)
  }

  function clearFilters() { setFilters({}) }

  function openNoteForRow(row: Record<string, unknown>) {
    setNoteInit({
      repo: rowRepo(row),
      ref_type: rowRefType(row, commands),
      ref_number: row.number !== undefined ? Number(row.number) : undefined,
      title: row.title ? String(row.title) : '',
    })
  }

  async function handleCreateNote(input: CreateNoteInput) {
    await api.notes.create(input)
    qc.invalidateQueries({ queryKey: ['notes'] })
    setNoteInit(null)
  }

  return (
    <div className="h-full flex flex-col gap-1">
      {/* Toolbar */}
      <div className="flex justify-end items-center gap-1 shrink-0 relative" ref={colPickerRef}>
        {errors.length > 0 && <span className="text-xs text-yellow-500 mr-auto">{errors.length} command(s) failed</span>}
        {activeFilterCount > 0 && (
          <button onClick={clearFilters} className="flex items-center gap-1 px-2 py-0.5 text-xs rounded text-yellow-400 hover:bg-gray-700">
            <X className="w-3 h-3" /> Clear filters ({activeFilterCount})
          </button>
        )}
        <button
          onClick={() => setShowFilters(v => !v)}
          className={`flex items-center gap-1 px-2 py-0.5 text-xs rounded ${showFilters || activeFilterCount > 0 ? 'bg-gray-700 text-gray-200' : 'text-gray-500 hover:text-gray-200 hover:bg-gray-700'}`}
        >
          <Filter className="w-3 h-3" />
          Filters{activeFilterCount > 0 ? ` (${activeFilterCount})` : ''}
        </button>
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
                <input type="checkbox" checked={!hiddenCols.has(k)} onChange={() => toggleColumn(k)} className="accent-blue-500" />
                {k}
              </label>
            ))}
          </div>
        )}
      </div>

      {/* Table */}
      <div className="overflow-auto flex-1">
        <table className="w-full text-xs">
          <thead>
            <tr className="border-b border-gray-700">
              <th className="py-1 px-2 text-gray-600 font-medium w-8 text-right select-none">#</th>
              {keys.map(k => (
                <th key={k} className="text-left py-1 px-2 text-gray-400 font-medium">
                  <span className="inline-flex items-center gap-1 w-full">
                    <span
                      onClick={() => handleSort(k)}
                      className="inline-flex items-center gap-1 cursor-pointer select-none hover:text-gray-200 capitalize"
                    >
                      {k}
                      {sortKey === k
                        ? sortDir === 'asc' ? <ChevronUp className="w-3 h-3" /> : <ChevronDown className="w-3 h-3" />
                        : <ChevronsUpDown className="w-3 h-3 opacity-30" />}
                    </span>
                    {showFilters && (
                      <button
                        onClick={e => openFilter(k, e)}
                        className={`ml-auto p-0.5 rounded shrink-0 ${
                          (filters[k]?.length ?? 0) > 0
                            ? 'text-blue-400 bg-blue-500/10'
                            : openFilterCol === k
                              ? 'text-gray-200 bg-gray-700'
                              : 'text-gray-600 hover:text-gray-300 hover:bg-gray-700'
                        }`}
                      >
                        <Filter className="w-3 h-3" />
                        {(filters[k]?.length ?? 0) > 0 && (
                          <span className="ml-0.5 text-[10px]">{filters[k].length}</span>
                        )}
                      </button>
                    )}
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
                <td className="py-1 px-2 text-gray-600 text-right tabular-nums w-8 shrink-0">{i + 1}</td>
                {keys.map(k => {
                  const val = row[k]
                  const url = row['url'] as string | undefined
                  const display = extractDisplay(val, extractors[k])
                  const tooltipText = extractors[k] && typeof val === 'object' && val !== null
                    ? JSON.stringify(val, null, 2) : display
                  const isActive = (filters[k] ?? []).includes(display)
                  const repo = rowRepo(row)
                  const cell = k === 'number' && url
                    ? <a href={url} target="_blank" rel="noopener noreferrer" className="text-blue-400 hover:text-blue-300 hover:underline tabular-nums">{display}</a>
                    : k === 'title' && url && repo
                    ? (
                      <Tooltip text={tooltipText}>
                        <span
                          onClick={() => setDetailItem({ repo, refType: rowRefType(row, commands), number: Number(row.number), url })}
                          className="cursor-pointer hover:text-blue-300 hover:underline"
                        >
                          {display}
                        </span>
                      </Tooltip>
                    )
                    : (
                      <Tooltip text={tooltipText}>
                        <span
                          onClick={() => handleCellClick(k, display)}
                          className={`cursor-pointer rounded px-0.5 ${isActive ? 'bg-blue-500/20 text-blue-300' : 'hover:bg-gray-600/50'}`}
                        >
                          {display}
                        </span>
                      </Tooltip>
                    )
                  return <td key={k} className="py-1 px-2 text-gray-300 max-w-xs truncate">{cell}</td>
                })}
                <td className="py-1 px-1 w-6">
                  {hoveredRow === i && (
                    <button onClick={() => openNoteForRow(row)} title="Add private note" className="p-0.5 rounded text-gray-500 hover:text-yellow-400 hover:bg-gray-600">
                      <StickyNote className="w-3.5 h-3.5" />
                    </button>
                  )}
                </td>
              </tr>
            ))}
            {rows.length === 0 && (
              <tr>
                <td colSpan={keys.length + 2} className="py-6 text-center text-gray-500">No rows match the current filters</td>
              </tr>
            )}
          </tbody>
        </table>
      </div>

      {openFilterCol && filterAnchor && (
        <FilterDropdown
          col={openFilterCol}
          values={columnValues[openFilterCol] ?? []}
          selected={filters[openFilterCol] ?? []}
          anchor={filterAnchor}
          onToggle={val => toggleFilterValue(openFilterCol, val)}
          onSelectAll={() => setFilters(prev => ({ ...prev, [openFilterCol]: [...(columnValues[openFilterCol] ?? [])] }))}
          onClear={() => setFilters(prev => ({ ...prev, [openFilterCol]: [] }))}
          onClose={() => setOpenFilterCol(null)}
        />
      )}

      {noteInit !== null && (
        <NoteDialog initialValues={noteInit} onConfirm={handleCreateNote} onClose={() => setNoteInit(null)} />
      )}
      {detailItem && (
        <DetailPanel
          repo={detailItem.repo}
          refType={detailItem.refType}
          number={detailItem.number}
          url={detailItem.url}
          onClose={() => setDetailItem(null)}
        />
      )}
    </div>
  )
}
