import type { CreateNoteInput, CreateTileInput, Dashboard, Note, Tile, TileLayout } from '../types'

const BASE = '/api'

async function req<T>(path: string, opts?: RequestInit): Promise<T> {
  const res = await fetch(`${BASE}${path}`, {
    headers: { 'Content-Type': 'application/json' },
    ...opts,
  })
  if (!res.ok) {
    const err = await res.json().catch(() => ({ error: res.statusText }))
    throw new Error(err.error || res.statusText)
  }
  if (res.status === 204) return undefined as T
  return res.json()
}

export const api = {
  notes: {
    list: (params?: { repo?: string; ref_type?: string; ref_number?: number }) => {
      const qs = params ? '?' + new URLSearchParams(Object.entries(params).filter(([,v]) => v !== undefined).map(([k,v]) => [k, String(v)])).toString() : ''
      return req<Note[]>(`/notes${qs}`)
    },
    get: (id: number) => req<Note>(`/notes/${id}`),
    create: (input: CreateNoteInput) => req<Note>('/notes', { method: 'POST', body: JSON.stringify(input) }),
    update: (id: number, input: Partial<CreateNoteInput>) => req<Note>(`/notes/${id}`, { method: 'PUT', body: JSON.stringify(input) }),
    delete: (id: number) => req<void>(`/notes/${id}`, { method: 'DELETE' }),
  },
  dashboards: {
    list: () => req<Dashboard[]>('/dashboards'),
    get: (id: number) => req<Dashboard>(`/dashboards/${id}`),
    create: (input: { name: string; description?: string }) => req<Dashboard>('/dashboards', { method: 'POST', body: JSON.stringify(input) }),
    update: (id: number, input: { name?: string; description?: string }) => req<Dashboard>(`/dashboards/${id}`, { method: 'PUT', body: JSON.stringify(input) }),
    delete: (id: number) => req<void>(`/dashboards/${id}`, { method: 'DELETE' }),
  },
  tiles: {
    list: (dashboardId: number) => req<Tile[]>(`/dashboards/${dashboardId}/tiles`),
    create: (dashboardId: number, input: CreateTileInput) => req<Tile>(`/dashboards/${dashboardId}/tiles`, { method: 'POST', body: JSON.stringify(input) }),
    update: (dashboardId: number, tileId: number, input: { title?: string; config?: object; layout?: TileLayout }) =>
      req<Tile>(`/dashboards/${dashboardId}/tiles/${tileId}`, { method: 'PUT', body: JSON.stringify(input) }),
    delete: (dashboardId: number, tileId: number) => req<void>(`/dashboards/${dashboardId}/tiles/${tileId}`, { method: 'DELETE' }),
  },
  gh: {
    execute: (command: string) => req<{ output: unknown; raw: string; cached: boolean }>('/gh/execute', { method: 'POST', body: JSON.stringify({ command }) }),
  },
  cache: {
    invalidate: () => req<{ invalidated_at: number }>('/cache/invalidate', { method: 'POST' }),
    status: () => req<{ cache_dir: string; ttl_secs: number; invalidated_at: number | null }>('/cache/status'),
  },
  backup: {
    export: () => req<{ dashboards: { id: number; name: string; description: string; tiles: { id: number; title: string; tile_type: string; config: unknown; layout: unknown }[] }[]; notes: unknown[]; version: number; exported_at: string }>('/backup'),
    restore: (data: unknown) => req<{ dashboards: { tile_ids: number[] }[] }>('/restore', { method: 'POST', body: JSON.stringify(data) }),
  },
  rowOrder: {
    get: (tileId: number) => req<{ order: string[] }>(`/tiles/${tileId}/row-order`),
    set: (tileId: number, order: string[]) => req<{ order: string[] }>(`/tiles/${tileId}/row-order`, { method: 'PUT', body: JSON.stringify({ order }) }),
  },
}
