export interface Note {
  id: number
  title: string
  body: string
  repo: string | null
  ref_type: 'issue' | 'pr' | 'repo' | null
  ref_number: number | null
  tags: string  // JSON array string
  created_at: string
  updated_at: string
}

export interface Dashboard {
  id: number
  name: string
  description: string
  created_at: string
  updated_at: string
}

export interface TileLayout {
  x: number
  y: number
  w: number
  h: number
}

export interface Tile {
  id: number
  dashboard_id: number
  title: string
  tile_type: 'gh_query' | 'note'
  config: string  // JSON string
  layout: string  // JSON string
  created_at: string
  updated_at: string
}

export interface GhQueryConfig {
  command: string
  columns?: string[]
  // Per-tile overrides: maps column name → sub-field to extract for display/sort.
  // Merged on top of DEFAULT_FIELD_EXTRACTORS. e.g. { "author": "login", "assignees": "0.login" }
  field_extractors?: Record<string, string>
}

export interface NoteConfig {
  note_id: number
}

export interface CreateNoteInput {
  title: string
  body?: string
  repo?: string
  ref_type?: 'issue' | 'pr' | 'repo'
  ref_number?: number
  tags?: string[]
}

export interface CreateTileInput {
  title: string
  tile_type: 'gh_query' | 'note'
  config: GhQueryConfig | NoteConfig
  layout?: TileLayout
}
