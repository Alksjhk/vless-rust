/**
 * 运行时长卡片组件
 */
import { ClockIcon } from '@heroicons/react/24/outline'
import MetricCard from './MetricCard'
import useMonitorStore from '../store/monitorStore'

export default function UptimeCard() {
  const { uptime } = useMonitorStore()

  return (
    <MetricCard
      title="运行时长"
      value={uptime}
      icon={ClockIcon}
      color="orange"
    />
  )
}
