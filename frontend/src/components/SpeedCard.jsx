/**
 * 速度卡片组件（增强版）
 */
import { ArrowUpTrayIcon, ArrowDownTrayIcon } from '@heroicons/react/24/outline'
import MetricCard from './MetricCard'
import useMonitorStore from '../store/monitorStore'

export default function SpeedCard() {
  const { uploadSpeed, downloadSpeed } = useMonitorStore()

  return (
    <div className="glass-card p-6">
      <h3 className="text-lg font-semibold text-gray-700 dark:text-gray-200 mb-4 flex items-center gap-2">
        <span className="w-2 h-2 rounded-full bg-blue-500 animate-pulse"></span>
        实时速度
      </h3>

      <div className="grid grid-cols-2 gap-4">
        {/* 上传速度 */}
        <MetricCard
          title="上传"
          value={uploadSpeed.split(' ')[0]}
          unit={uploadSpeed.split(' ')[1]}
          icon={ArrowUpTrayIcon}
          color="blue"
          className="p-4"
        />

        {/* 下载速度 */}
        <MetricCard
          title="下载"
          value={downloadSpeed.split(' ')[0]}
          unit={downloadSpeed.split(' ')[1]}
          icon={ArrowDownTrayIcon}
          color="green"
          className="p-4"
        />
      </div>
    </div>
  )
}
