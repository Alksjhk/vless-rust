import { Users } from 'lucide-react'
import MetricCard from './MetricCard'
import { Progress } from '../ui/progress'
import useMonitorStore from '../../store/monitorStore'
import { parseSpeedString } from '../../utils/formatters'

export default function ConnectionsMetric() {
  const { activeConnections, maxConnections } = useMonitorStore()

  const connectionPercent = maxConnections > 0
    ? Math.round((activeConnections / maxConnections) * 100)
    : 0

  const getConnectionStatus = (percent) => {
    if (percent < 50) return { variant: 'success', text: '正常' }
    if (percent < 80) return { variant: 'warning', text: '繁忙' }
    return { variant: 'destructive', text: '告警' }
  }

  const status = getConnectionStatus(connectionPercent)

  return (
    <MetricCard
      title="活跃连接"
      value={
        <div className="flex items-baseline gap-2">
          <span className="text-2xl font-bold">{activeConnections || 0}</span>
          <span className="text-sm text-muted-foreground">/ {maxConnections || 0}</span>
        </div>
      }
      icon={Users}
      description={
        <div className="mt-2 space-y-1">
          <div className="flex items-center justify-between text-xs">
            <span className="text-muted-foreground">使用率</span>
            <span className="font-medium">{connectionPercent}%</span>
          </div>
          <Progress value={connectionPercent} className="h-1.5" />
        </div>
      }
    />
  )
}
