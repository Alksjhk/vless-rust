/**
 * 系统信息面板组件（增强版）
 */
import { ServerIcon, GlobeAltIcon, ClockIcon } from '@heroicons/react/24/outline'
import useMonitorStore from '../store/monitorStore'
import { memo } from 'react'

// 颜色类配置（移到组件外部，避免每次渲染重新创建）
const colorClasses = {
  green: {
    bg: 'bg-green-500',
    gradient: 'from-green-50 to-emerald-50 dark:from-green-900/20 dark:to-emerald-900/20',
    border: 'border-green-100 dark:border-green-800',
    text: 'text-green-600 dark:text-green-400',
  },
  blue: {
    bg: 'bg-blue-500',
    gradient: 'from-blue-50 to-cyan-50 dark:from-blue-900/20 dark:to-cyan-900/20',
    border: 'border-blue-100 dark:border-blue-800',
    text: 'text-blue-600 dark:text-blue-400',
  },
  yellow: {
    bg: 'bg-yellow-500',
    gradient: 'from-yellow-50 to-amber-50 dark:from-yellow-900/20 dark:to-amber-900/20',
    border: 'border-yellow-100 dark:border-yellow-800',
    text: 'text-yellow-600 dark:text-yellow-400',
  },
  red: {
    bg: 'bg-red-500',
    gradient: 'from-red-50 to-rose-50 dark:from-red-900/20 dark:to-rose-900/20',
    border: 'border-red-100 dark:border-red-800',
    text: 'text-red-600 dark:text-red-400',
  },
}

const SystemInfo = memo(function SystemInfo() {
  const { maxConnections, uptime, publicIp } = useMonitorStore()

  const stats = [
    {
      label: '运行状态',
      value: '正常运行',
      icon: ServerIcon,
      color: 'green',
    },
    {
      label: '公网IP',
      value: publicIp,
      icon: GlobeAltIcon,
      color: 'blue',
    },
  ]

  return (
    <div className="glass-card p-6 h-full">
      <h3 className="text-xl font-bold text-gray-800 dark:text-gray-100 mb-6">系统信息</h3>

      <div className="space-y-4">
        {stats.map((stat, index) => {
          const Icon = stat.icon
          const colors = colorClasses[stat.color] || colorClasses.blue

          return (
            <div
              key={index}
              className="flex items-center gap-4 p-3 glass-subtle rounded-xl hover:bg-white/40 dark:hover:bg-white/10 transition-all btn-hover"
            >
              <div className={`p-2 rounded-lg ${colors.bg} shadow-md`}>
                <Icon className="w-5 h-5 text-white" />
              </div>
              <div className="flex-1">
                <p className="text-sm text-gray-500 dark:text-gray-400">{stat.label}</p>
                <p className="text-lg font-bold text-gray-800 dark:text-gray-100">{stat.value}</p>
                {stat.percent !== undefined && (
                  <div className="mt-2">
                    <div className="progress-bar">
                      <div
                        className={`h-full bg-gradient-to-r ${colors.gradient} transition-all duration-500 ease-out rounded-full`}
                        style={{ width: `${Math.min(stat.percent, 100)}%` }}
                      />
                    </div>
                  </div>
                )}
              </div>
            </div>
          )
        })}

        {/* 运行时长 */}
        <div className={`mt-6 p-4 bg-gradient-to-br ${colorClasses.blue.gradient} rounded-xl border ${colorClasses.blue.border} transition-all duration-300 hover:shadow-lg`}>
          <div className="flex items-center gap-3">
            <ClockIcon className="w-6 h-6 text-blue-500" />
            <div>
              <p className="text-sm text-gray-600 dark:text-gray-400 mb-1">运行时长</p>
              <p className="text-2xl font-bold text-gray-800 dark:text-gray-100">{uptime}</p>
            </div>
          </div>
        </div>

        {/* 最大连接数 */}
        <div className={`p-4 bg-gradient-to-br from-purple-50 to-pink-50 dark:from-purple-900/20 dark:to-pink-900/20 rounded-xl border border-purple-100 dark:border-purple-800 transition-all duration-300 hover:shadow-lg`}>
          <p className="text-sm text-gray-600 dark:text-gray-400 mb-1">最大连接数</p>
          <p className="text-2xl font-bold text-gray-800 dark:text-gray-100">{maxConnections}</p>
        </div>
      </div>
    </div>
  )
})

export default SystemInfo
