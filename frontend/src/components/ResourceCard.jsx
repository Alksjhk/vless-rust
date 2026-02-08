/**
 * 资源使用卡片组件 (CPU/内存)
 */
import { CpuChipIcon, ServerIcon } from '@heroicons/react/24/outline'
import useMonitorStore from '../store/monitorStore'

export default function ResourceCard() {
  const { memoryUsage, activeConnections, maxConnections, getMemoryPercent } = useMonitorStore()

  const memoryPercent = getMemoryPercent()
  const connectionPercent = Math.round((activeConnections / maxConnections) * 100)

  return (
    <div className="glass-card p-6">
      <h3 className="text-lg font-semibold text-gray-700 mb-4">系统资源</h3>

      <div className="space-y-4">
        {/* 内存使用 */}
        <div>
          <div className="flex items-center justify-between mb-2">
            <div className="flex items-center gap-2">
              <ServerIcon className="w-5 h-5 text-purple-500" />
              <span className="text-sm font-medium text-gray-600">内存使用</span>
            </div>
            <span className="text-sm font-bold text-gray-800">
              {memoryUsage}
            </span>
          </div>
          <div className="w-full bg-gray-200 rounded-full h-2 overflow-hidden">
            <div
              className={`h-full bg-gradient-to-r ${
                memoryPercent < 50
                  ? 'from-green-400 to-green-500'
                  : memoryPercent < 75
                  ? 'from-yellow-400 to-yellow-500'
                  : 'from-red-400 to-red-500'
              } transition-all duration-500`}
              style={{ width: `${Math.min(memoryPercent, 100)}%` }}
            />
          </div>
          <p className="text-xs text-gray-500 mt-1">{memoryUsage} 已使用</p>
        </div>

        {/* 连接数 */}
        <div>
          <div className="flex items-center justify-between mb-2">
            <div className="flex items-center gap-2">
              <CpuChipIcon className="w-5 h-5 text-blue-500" />
              <span className="text-sm font-medium text-gray-600">活跃连接</span>
            </div>
            <span className="text-sm font-bold text-gray-800">
              {activeConnections} / {maxConnections}
            </span>
          </div>
          <div className="w-full bg-gray-200 rounded-full h-2 overflow-hidden">
            <div
              className={`h-full bg-gradient-to-r ${
                connectionPercent < 50
                  ? 'from-blue-400 to-blue-500'
                  : connectionPercent < 75
                  ? 'from-yellow-400 to-yellow-500'
                  : 'from-red-400 to-red-500'
              } transition-all duration-500`}
              style={{ width: `${Math.min(connectionPercent, 100)}%` }}
            />
          </div>
          <p className="text-xs text-gray-500 mt-1">{connectionPercent}% 使用率</p>
        </div>
      </div>
    </div>
  )
}
