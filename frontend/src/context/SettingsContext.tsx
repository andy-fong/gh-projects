import { createContext, useContext, useState, useCallback } from 'react'
import { DEFAULT_FIELD_EXTRACTORS } from '../lib/fieldExtractors'

interface GlobalSettings {
  field_extractors: Record<string, string>
}

const STORAGE_KEY = 'gh-projects-global-settings'

function loadSettings(): GlobalSettings {
  try {
    const stored = localStorage.getItem(STORAGE_KEY)
    return stored ? JSON.parse(stored) : { field_extractors: {} }
  } catch {
    return { field_extractors: {} }
  }
}

interface SettingsContextValue {
  settings: GlobalSettings
  // Merged extractors ready to use: defaults → global → (per-tile applied at call site)
  globalExtractors: Record<string, string>
  updateSettings: (next: Partial<GlobalSettings>) => void
}

const SettingsContext = createContext<SettingsContextValue | null>(null)

export function SettingsProvider({ children }: { children: React.ReactNode }) {
  const [settings, setSettings] = useState<GlobalSettings>(loadSettings)

  const globalExtractors = { ...DEFAULT_FIELD_EXTRACTORS, ...settings.field_extractors }

  const updateSettings = useCallback((next: Partial<GlobalSettings>) => {
    setSettings(prev => {
      const updated = { ...prev, ...next }
      localStorage.setItem(STORAGE_KEY, JSON.stringify(updated))
      return updated
    })
  }, [])

  return (
    <SettingsContext.Provider value={{ settings, globalExtractors, updateSettings }}>
      {children}
    </SettingsContext.Provider>
  )
}

export function useSettings() {
  const ctx = useContext(SettingsContext)
  if (!ctx) throw new Error('useSettings must be used within SettingsProvider')
  return ctx
}
