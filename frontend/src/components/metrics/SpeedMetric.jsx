import { ArrowUp, ArrowDown } from 'lucide-react'
import MetricCard from './MetricCard'
import useMonitorStore from '../../store/monitorStore'

export default function SpeedMetric() {
  const { uploadSpeed, downloadSpeed } = useMonitorStore()

  return (
    <MetricCard
      title="实时速度"
      value={
        <div className="flex flex-col gap-1">
          <div className="flex items-center gap-2">
            <ArrowUp className="h-4 w-4 text-blue-500" />
            <span className="text-lg">{uploadSpeed || '0 B/s'}</span>
          </div>
          <div className="flex items-center gap-2">
            <ArrowDown className="h-4 w-4 text-green-500" />
            <span className="text-lg">{downloadSpeed || '0 B/s'}</span>
          </div>
        </div>
      }
      description="上传 / 下载速度"
    />
  )
}
