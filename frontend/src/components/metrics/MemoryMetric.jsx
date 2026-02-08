import { MemoryStick } from 'lucide-react'
import MetricCard from './MetricCard'
import useMonitorStore from '../../store/monitorStore'
import { memo } from 'react'

const MemoryMetric = memo(function MemoryMetric() {
  const { memoryUsage } = useMonitorStore()

  return (
    <MetricCard
      title="内存使用"
      value={<span className="text-2xl font-bold">{memoryUsage || '0 B'}</span>}
      icon={MemoryStick}
      description={
        <div className="mt-2">
          <span className="text-xs text-muted-foreground">已使用 {memoryUsage || '0 B'}</span>
        </div>
      }
    />
  )
})

export default MemoryMetric
