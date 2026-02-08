import { Clock } from 'lucide-react'
import MetricCard from './MetricCard'
import useMonitorStore from '../../store/monitorStore'

export default function UptimeMetric() {
  const { uptime } = useMonitorStore()

  return (
    <MetricCard
      title="运行时间"
      value={uptime || '0s'}
      icon={Clock}
      description="服务器持续运行时长"
    />
  )
}
