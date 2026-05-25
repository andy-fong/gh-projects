import { Routes, Route, Navigate } from 'react-router-dom'
import { useQuery } from '@tanstack/react-query'
import { api } from './api/client'
import { SettingsProvider } from './context/SettingsContext'
import Sidebar from './components/Sidebar'
import DashboardPage from './pages/DashboardPage'
import NotesPage from './pages/NotesPage'

export default function App() {
  const { data: dashboards = [] } = useQuery({
    queryKey: ['dashboards'],
    queryFn: api.dashboards.list,
  })

  return (
    <SettingsProvider>
      <div className="flex h-screen overflow-hidden">
        <Sidebar dashboards={dashboards} />
        <main className="flex-1 overflow-auto">
          <Routes>
            <Route path="/" element={dashboards[0] ? <Navigate to={`/dashboards/${dashboards[0].id}`} /> : <div className="p-8 text-gray-400">Create a dashboard to get started.</div>} />
            <Route path="/dashboards/:id" element={<DashboardPage />} />
            <Route path="/notes" element={<NotesPage />} />
          </Routes>
        </main>
      </div>
    </SettingsProvider>
  )
}
