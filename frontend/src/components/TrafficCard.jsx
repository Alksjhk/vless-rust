/**
 * 总流量卡片组件
 */
import { CloudArrowUpIcon } from '@heroicons/react/24/outline'
import MetricCard from './MetricCard'
import useMonitorStore from '../store/monitorStore'

export default function TrafficCard() {
  const { totalTraffic } = useMonitorStore()

  return (
    <MetricCard
      title="总流量"
      value={totalTraffic.split(' ')[0]}
      unit={totalTraffic.split(' ')[1]}
      icon={CloudArrowUpIcon}
      color="purple"
    />
  )
}
