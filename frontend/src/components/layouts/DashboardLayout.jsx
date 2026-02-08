import Header from './Header'
import useMonitorStore from '../../store/monitorStore'

export default function DashboardLayout({ children }) {
  const { isConnected } = useMonitorStore()

  return (
    <div className="min-h-screen bg-background">
      <Header connectionStatus={isConnected ? 'connected' : 'disconnected'} />

      <main className="container mx-auto px-4 py-6">
        {children}
      </main>
    </div>
  )
}
