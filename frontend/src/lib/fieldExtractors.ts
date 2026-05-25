// Maps a column name to the sub-field(s) to display when the value is an object.
// Use "|" to specify fallbacks: "name|login" tries "name" first, falls back to "login".
// Add entries here to extend defaults, or override per-tile via the tile config.
export const DEFAULT_FIELD_EXTRACTORS: Record<string, string> = {
  author: 'login',
  repository: 'nameWithOwner',
  assignees: 'name|login',
}

function pickField(obj: Record<string, unknown>, extractor: string): string {
  for (const field of extractor.split('|')) {
    const val = obj[field.trim()]
    if (val != null && val !== '') return String(val)
  }
  return ''
}

export function extractDisplay(val: unknown, extractor?: string): string {
  if (val === null || val === undefined) return ''

  if (Array.isArray(val)) {
    if (val.length === 0) return ''
    return val
      .map(item => {
        if (extractor && typeof item === 'object' && item !== null)
          return pickField(item as Record<string, unknown>, extractor)
        return typeof item === 'object' ? JSON.stringify(item) : String(item)
      })
      .filter(Boolean)
      .join(', ')
  }

  if (extractor && typeof val === 'object') {
    const result = pickField(val as Record<string, unknown>, extractor)
    return result !== '' ? result : JSON.stringify(val)
  }
  if (typeof val === 'object') return JSON.stringify(val)
  return String(val)
}
