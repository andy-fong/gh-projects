// Maps a column name to the sub-field to display when the value is an object.
// Add entries here to extend defaults, or override per-tile via the tile config.
export const DEFAULT_FIELD_EXTRACTORS: Record<string, string> = {
  author: 'login',
  repository: 'nameWithOwner',
}

export function extractDisplay(val: unknown, extractor?: string): string {
  if (val === null || val === undefined) return ''
  if (extractor && typeof val === 'object') {
    const nested = (val as Record<string, unknown>)[extractor]
    return nested !== undefined ? String(nested) : JSON.stringify(val)
  }
  if (typeof val === 'object') return JSON.stringify(val)
  return String(val)
}
