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
  command: string        // single command (legacy / simple case)
  commands?: string[]    // multiple commands — results are merged into one table
  columns?: string[]
  field_extractors?: Record<string, string>
  variables?: Record<string, string | string[]>  // template vars; array value → one run per element
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
