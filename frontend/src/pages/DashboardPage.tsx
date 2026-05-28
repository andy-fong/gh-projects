import { useState, useRef, useEffect } from 'react'
import { useParams } from 'react-router-dom'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import GridLayout from 'react-grid-layout'
import type { Layout } from 'react-grid-layout'
import { Plus, ClipboardPaste } from 'lucide-react'
import { api } from '../api/client'
import TileWrapper from '../components/TileWrapper'
import GhQueryTile from '../components/tiles/GhQueryTile'
import NoteTile from '../components/tiles/NoteTile'
import NewTileDialog, { type NewTileInitialValues } from '../components/dialogs/NewTileDialog'
import EditTileDialog from '../components/dialogs/EditTileDialog'
import type { CreateTileInput, Tile, GhQueryConfig, NoteConfig } from '../types'

export default function DashboardPage() {
  const { id } = useParams<{ id: string }>()
  const dashboardId = Number(id)
  const qc = useQueryClient()

  const { data: dashboard } = useQuery({ queryKey: ['dashboard', dashboardId], queryFn: () => api.dashboards.get(dashboardId) })
  const { data: tiles = [] } = useQuery({ queryKey: ['tiles', dashboardId], queryFn: () => api.tiles.list(dashboardId) })

  const [showNewTile, setShowNewTile] = useState(false)
  const [newTileInitialValues, setNewTileInitialValues] = useState<NewTileInitialValues | undefined>(undefined)
  const [copiedTile, setCopiedTile] = useState<Tile | null>(null)
  const [editingTile, setEditingTile] = useState<Tile | null>(null)
  const [renamingDashboard, setRenamingDashboard] = useState(false)
  const [renameValue, setRenameValue] = useState('')
  const renameInputRef = useRef<HTMLInputElement>(null)

  useEffect(() => {
    if (renamingDashboard) renameInputRef.current?.select()
  }, [renamingDashboard])

  function startRename() {
    setRenameValue(dashboard?.name ?? '')
    setRenamingDashboard(true)
  }

  async function commitRename() {
    const name = renameValue.trim()
    if (name && name !== dashboard?.name) {
      await api.dashboards.update(dashboardId, { name })
      qc.invalidateQueries({ queryKey: ['dashboard', dashboardId] })
      qc.invalidateQueries({ queryKey: ['dashboards'] })
    }
    setRenamingDashboard(false)
  }

  function handleRenameKey(e: React.KeyboardEvent) {
    if (e.key === 'Enter') commitRename()
    if (e.key === 'Escape') setRenamingDashboard(false)
  }

  function getTilePasteValues(tile: Tile): NewTileInitialValues {
    const config = JSON.parse(tile.config)
    if (tile.tile_type === 'gh_query') {
      const ghConfig = config as GhQueryConfig
      const vars = ghConfig.variables ?? {}
      return {
        type: 'gh_query',
        title: `${tile.title} (copy)`,
        commands: ghConfig.commands ?? (ghConfig.command ? [ghConfig.command] : ['']),
        variablesJson: Object.keys(vars).length > 0 ? JSON.stringify(vars, null, 2) : '',
      }
    } else {
      const noteConfig = config as NoteConfig
      return {
        type: 'note',
        title: `${tile.title} (copy)`,
        noteId: String(noteConfig.note_id),
      }
    }
  }

  async function handleAddTile(input: CreateTileInput) {
    await api.tiles.create(dashboardId, input)
    qc.invalidateQueries({ queryKey: ['tiles', dashboardId] })
    setShowNewTile(false)
    setNewTileInitialValues(undefined)
  }

  function openPasteTile() {
    if (!copiedTile) return
    setNewTileInitialValues(getTilePasteValues(copiedTile))
    setShowNewTile(true)
  }

  async function handleDeleteTile(tileId: number) {
    await api.tiles.delete(dashboardId, tileId)
    qc.invalidateQueries({ queryKey: ['tiles', dashboardId] })
  }

  async function handleEditTile(update: { title?: string; config?: object }) {
    if (!editingTile) return
    await api.tiles.update(dashboardId, editingTile.id, update)
    qc.invalidateQueries({ queryKey: ['tiles', dashboardId] })
    setEditingTile(null)
  }

  async function handleLayoutChange(layouts: Layout[]) {
    await Promise.all(
      layouts.map(l => {
        const tile = tiles.find(t => String(t.id) === l.i)
        if (!tile) return Promise.resolve()
        return api.tiles.update(dashboardId, tile.id, {
          layout: { x: l.x, y: l.y, w: l.w, h: l.h },
        })
      })
    )
  }

  const layout: Layout[] = tiles.map(t => {
    const l = JSON.parse(t.layout)
    return { i: String(t.id), x: l.x, y: l.y, w: l.w, h: l.h }
  })

  return (
    <div className="p-4">
      <div className="flex items-center justify-between mb-4">
        {renamingDashboard ? (
          <input
            ref={renameInputRef}
            value={renameValue}
            onChange={e => setRenameValue(e.target.value)}
            onBlur={commitRename}
            onKeyDown={handleRenameKey}
            className="text-lg font-semibold bg-transparent border-b border-blue-500 outline-none text-white"
          />
        ) : (
          <h1
            className="text-lg font-semibold cursor-pointer hover:text-blue-400"
            onClick={startRename}
            title="Click to rename"
          >
            {dashboard?.name}
          </h1>
        )}
        <div className="flex items-center gap-2">
          {copiedTile && (
            <button
              onClick={openPasteTile}
              className="flex items-center gap-2 px-3 py-1.5 text-sm bg-gray-700 hover:bg-gray-600 border border-gray-600 rounded text-white"
              title={`Paste "${copiedTile.title}"`}
            >
              <ClipboardPaste className="w-4 h-4" /> Paste tile
            </button>
          )}
          <button
            onClick={() => { setNewTileInitialValues(undefined); setShowNewTile(true) }}
            className="flex items-center gap-2 px-3 py-1.5 text-sm bg-blue-600 hover:bg-blue-500 rounded text-white"
          >
            <Plus className="w-4 h-4" /> Add tile
          </button>
        </div>
      </div>

      {tiles.length === 0 ? (
        <div className="text-gray-500 text-sm mt-16 text-center">
          No tiles yet. Add a tile to get started.
        </div>
      ) : (
        <GridLayout
          className="layout"
          layout={layout}
          cols={12}
          rowHeight={60}
          width={window.innerWidth - 256}
          onLayoutChange={handleLayoutChange}
          draggableHandle=".drag-handle"
        >
          {tiles.map(tile => {
            const config = JSON.parse(tile.config)
            return (
              <div key={String(tile.id)}>
                <TileWrapper
                  tile={tile}
                  onDelete={() => handleDeleteTile(tile.id)}
                  onEdit={() => setEditingTile(tile)}
                  onCopy={() => setCopiedTile(tile)}
                >
                  {tile.tile_type === 'gh_query' && <GhQueryTile config={config} tileId={tile.id} />}
                  {tile.tile_type === 'note' && <NoteTile config={config} />}
                </TileWrapper>
              </div>
            )
          })}
        </GridLayout>
      )}

      {showNewTile && (
        <NewTileDialog
          onConfirm={handleAddTile}
          onClose={() => { setShowNewTile(false); setNewTileInitialValues(undefined) }}
          initialValues={newTileInitialValues}
        />
      )}
      {editingTile && <EditTileDialog tile={editingTile} onConfirm={handleEditTile} onClose={() => setEditingTile(null)} />}
    </div>
  )
}
