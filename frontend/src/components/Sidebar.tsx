import { useState } from 'react'
import { Link, useParams } from 'react-router-dom'
import { useQueryClient } from '@tanstack/react-query'
import { LayoutDashboard, StickyNote, Plus, Github, Settings, RefreshCcw } from 'lucide-react'
import { api } from '../api/client'
import SettingsDialog from './dialogs/SettingsDialog'
import type { Dashboard } from '../types'

interface Props {
  dashboards: Dashboard[]
}

export default function Sidebar({ dashboards }: Props) {
  const { id } = useParams()
  const qc = useQueryClient()
  const [creating, setCreating] = useState(false)
  const [newName, setNewName] = useState('')
  const [showSettings, setShowSettings] = useState(false)
  const [invalidating, setInvalidating] = useState(false)

  async function handleInvalidateCache() {
    setInvalidating(true)
    try {
      await api.cache.invalidate()
      qc.invalidateQueries({ queryKey: ['gh'] })
    } finally {
      setInvalidating(false)
    }
  }

  async function handleCreate(e: React.FormEvent) {
    e.preventDefault()
    if (!newName.trim()) return
    await api.dashboards.create({ name: newName.trim() })
    qc.invalidateQueries({ queryKey: ['dashboards'] })
    setNewName('')
    setCreating(false)
  }

  return (
    <aside className="w-56 bg-gray-900 border-r border-gray-700 flex flex-col shrink-0">
      <div className="px-4 py-4 border-b border-gray-700 flex items-center gap-2">
        <Github className="w-5 h-5 text-blue-400" />
        <span className="font-semibold text-sm">GH Projects</span>
      </div>
      <nav className="flex-1 overflow-y-auto p-2">
        <div className="text-xs text-gray-500 uppercase tracking-wider px-2 py-1 mt-2">Dashboards</div>
        {dashboards.map(d => (
          <Link
            key={d.id}
            to={`/dashboards/${d.id}`}
            className={`flex items-center gap-2 px-2 py-1.5 rounded text-sm hover:bg-gray-700 ${id === String(d.id) ? 'bg-gray-700 text-white' : 'text-gray-300'}`}
          >
            <LayoutDashboard className="w-4 h-4" />
            {d.name}
          </Link>
        ))}
        {creating ? (
          <form onSubmit={handleCreate} className="mt-1">
            <input
              autoFocus
              value={newName}
              onChange={e => setNewName(e.target.value)}
              onBlur={() => setCreating(false)}
              className="w-full px-2 py-1 text-sm bg-gray-800 border border-gray-600 rounded text-white outline-none"
              placeholder="Dashboard name…"
            />
          </form>
        ) : (
          <button
            onClick={() => setCreating(true)}
            className="flex items-center gap-2 px-2 py-1.5 rounded text-sm text-gray-500 hover:text-gray-300 hover:bg-gray-700 w-full mt-1"
          >
            <Plus className="w-4 h-4" /> New dashboard
          </button>
        )}

        <div className="text-xs text-gray-500 uppercase tracking-wider px-2 py-1 mt-4">Tools</div>
        <Link
          to="/notes"
          className="flex items-center gap-2 px-2 py-1.5 rounded text-sm text-gray-300 hover:bg-gray-700"
        >
          <StickyNote className="w-4 h-4" /> Notes
        </Link>
      </nav>
      <div className="p-2 border-t border-gray-700 space-y-1">
        <button
          onClick={handleInvalidateCache}
          disabled={invalidating}
          className="flex items-center gap-2 px-2 py-1.5 rounded text-sm text-gray-500 hover:text-gray-300 hover:bg-gray-700 w-full disabled:opacity-50"
          title="Invalidate all cached gh CLI results"
        >
          <RefreshCcw className={`w-4 h-4 ${invalidating ? 'animate-spin' : ''}`} />
          {invalidating ? 'Invalidating…' : 'Invalidate cache'}
        </button>
        <button
          onClick={() => setShowSettings(true)}
          className="flex items-center gap-2 px-2 py-1.5 rounded text-sm text-gray-500 hover:text-gray-300 hover:bg-gray-700 w-full"
        >
          <Settings className="w-4 h-4" /> Settings
        </button>
      </div>
      {showSettings && <SettingsDialog onClose={() => setShowSettings(false)} />}
    </aside>
  )
}
