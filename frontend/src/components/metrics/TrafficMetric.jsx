import { HardDrive } from 'lucide-react'
import MetricCard from './MetricCard'
import useMonitorStore from '../../store/monitorStore'

export default function TrafficMetric() {
  const { totalTraffic } = useMonitorStore()

  return (
    <MetricCard
      title="总流量"
      value={totalTraffic || '0 B'}
      icon={HardDrive}
      description="累计传输数据总量"
    />
  )
}
