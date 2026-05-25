import { useState } from 'react'
import { Trash2, Settings } from 'lucide-react'
import type { Tile } from '../types'

interface Props {
  tile: Tile
  onDelete: () => void
  onEdit: () => void
  children: React.ReactNode
}

export default function TileWrapper({ tile, onDelete, onEdit, children }: Props) {
  const [hovered, setHovered] = useState(false)

  return (
    <div
      className="bg-gray-800 border border-gray-700 rounded-lg overflow-hidden h-full flex flex-col"
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
    >
      <div className="flex items-center justify-between px-3 py-2 border-b border-gray-700 shrink-0">
        <span className="text-sm font-medium text-gray-200 truncate">{tile.title}</span>
        {hovered && (
          <div className="flex gap-1">
            <button onClick={onEdit} className="p-1 rounded hover:bg-gray-700 text-gray-400 hover:text-gray-200">
              <Settings className="w-3.5 h-3.5" />
            </button>
            <button onClick={onDelete} className="p-1 rounded hover:bg-gray-700 text-gray-400 hover:text-red-400">
              <Trash2 className="w-3.5 h-3.5" />
            </button>
          </div>
        )}
      </div>
      <div className="flex-1 overflow-auto p-3">{children}</div>
    </div>
  )
}
