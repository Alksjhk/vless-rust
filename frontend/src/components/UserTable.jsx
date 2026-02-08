/**
 * 用户流量表格组件（增强版 - 支持移动端卡片视图）
 */
import { useState } from 'react'
import { ArrowUpTrayIcon, ArrowDownTrayIcon, ChevronDownIcon, ChevronUpIcon } from '@heroicons/react/24/outline'
import useMonitorStore from '../store/monitorStore'
import { parseSpeedString, parseTrafficString, getProgressColor } from '../utils/formatters'

/**
 * 用户卡片组件（移动端）
 */
function UserCard({ user, trafficPercent, progressColor }) {
  const [isExpanded, setIsExpanded] = useState(false)

  return (
    <div className="glass-subtle rounded-xl p-4 mb-3 hover:bg-white/40 dark:hover:bg-white/10 transition-all">
      {/* 用户基本信息 */}
      <div className="flex items-center justify-between mb-3">
        <div className="flex-1">
          <p className="font-medium text-gray-800 dark:text-gray-100 font-mono text-sm">
            {user.email || 'N/A'}
          </p>
          <p className="text-xs text-gray-500 dark:text-gray-400 font-mono mt-1">
            {user.uuid?.slice(0, 8)}...
          </p>
        </div>
        <button
          onClick={() => setIsExpanded(!isExpanded)}
          className="p-2 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors"
        >
          {isExpanded ? (
            <ChevronUpIcon className="w-5 h-5 text-gray-500 dark:text-gray-400" />
          ) : (
            <ChevronDownIcon className="w-5 h-5 text-gray-500 dark:text-gray-400" />
          )}
        </button>
      </div>

      {/* 总流量和进度条 */}
      <div className="mb-3">
        <div className="flex items-center justify-between mb-1">
          <span className="text-sm text-gray-500 dark:text-gray-400">总流量</span>
          <span className="text-xs text-gray-500 dark:text-gray-400">{trafficPercent.toFixed(1)}%</span>
        </div>
        <div className="w-full progress-bar">
          <div
            className={`h-full bg-gradient-to-r ${progressColor} transition-all duration-500`}
            style={{ width: `${Math.min(trafficPercent, 100)}%` }}
          />
        </div>
        <p className="text-right mt-1 font-semibold text-gray-800 dark:text-gray-100">
          {user.total_traffic}
        </p>
      </div>

      {/* 详细信息（可展开） */}
      {isExpanded && (
        <div className="grid grid-cols-2 gap-3 pt-3 border-t border-gray-200 dark:border-gray-700 animate-fade-in">
          {/* 上传速度 */}
          <div className="flex items-center gap-2 p-2 bg-blue-50 dark:bg-blue-900/20 rounded-lg">
            <ArrowUpTrayIcon className="w-4 h-4 text-blue-500" />
            <div>
              <p className="text-xs text-gray-500 dark:text-gray-400">上传</p>
              <p className="text-sm font-medium text-gray-800 dark:text-gray-100">{user.upload_speed}</p>
            </div>
          </div>

          {/* 下载速度 */}
          <div className="flex items-center gap-2 p-2 bg-green-50 dark:bg-green-900/20 rounded-lg">
            <ArrowDownTrayIcon className="w-4 h-4 text-green-500" />
            <div>
              <p className="text-xs text-gray-500 dark:text-gray-400">下载</p>
              <p className="text-sm font-medium text-gray-800 dark:text-gray-100">{user.download_speed}</p>
            </div>
          </div>

          {/* 连接数 */}
          <div className="col-span-2 flex items-center justify-between p-2 bg-purple-50 dark:bg-purple-900/20 rounded-lg">
            <span className="text-sm text-gray-500 dark:text-gray-400">活跃连接</span>
            <span className="inline-flex items-center justify-center px-3 py-1 rounded-full text-sm font-medium bg-blue-100 dark:bg-blue-900/30 text-blue-800 dark:text-blue-300">
              {user.active_connections}
            </span>
          </div>
        </div>
      )}
    </div>
  )
}

export default function UserTable() {
  const { users } = useMonitorStore()

  // 计算总流量以显示进度条
  const totalTraffic = users.reduce((sum, user) => {
    return sum + parseTrafficString(user.total_traffic)
  }, 0)

  return (
    <div className="glass-card p-6">
      <div className="flex items-center justify-between mb-6">
        <div>
          <h3 className="text-xl font-bold text-gray-800 dark:text-gray-100">用户流量统计</h3>
          <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">
            共 {users.length} 个用户
          </p>
        </div>
      </div>

      {users.length === 0 ? (
        <div className="text-center py-12 text-gray-500 dark:text-gray-400">
          <p>暂无用户数据</p>
        </div>
      ) : (
        <>
          {/* 桌面端表格视图 */}
          <div className="hidden md:block overflow-x-auto">
            <table className="w-full">
              <thead>
                <tr className="border-b border-gray-200 dark:border-gray-700">
                  <th className="text-left py-3 px-4 font-semibold text-gray-700 dark:text-gray-300">用户</th>
                  <th className="text-left py-3 px-4 font-semibold text-gray-700 dark:text-gray-300">上传速度</th>
                  <th className="text-left py-3 px-4 font-semibold text-gray-700 dark:text-gray-300">下载速度</th>
                  <th className="text-left py-3 px-4 font-semibold text-gray-700 dark:text-gray-300">总流量</th>
                  <th className="text-left py-3 px-4 font-semibold text-gray-700 dark:text-gray-300">流量占比</th>
                  <th className="text-center py-3 px-4 font-semibold text-gray-700 dark:text-gray-300">连接数</th>
                </tr>
              </thead>
              <tbody>
                {users.map((user, index) => {
                  const userTraffic = parseTrafficString(user.total_traffic)
                  const trafficPercent = totalTraffic > 0 ? (userTraffic / totalTraffic) * 100 : 0
                  const progressColor = getProgressColor(trafficPercent)

                  return (
                    <tr
                      key={user.uuid || index}
                      className="border-b border-gray-100 dark:border-gray-800 hover:bg-white/40 dark:hover:bg-white/10 transition-colors"
                    >
                      {/* 用户信息 */}
                      <td className="py-4 px-4">
                        <div>
                          <p className="font-medium text-gray-800 dark:text-gray-100 font-mono text-sm">
                            {user.email || 'N/A'}
                          </p>
                          <p className="text-xs text-gray-500 dark:text-gray-400 font-mono mt-1">
                            {user.uuid?.slice(0, 8)}...
                          </p>
                        </div>
                      </td>

                      {/* 上传速度 */}
                      <td className="py-4 px-4">
                        <div className="flex items-center gap-2">
                          <ArrowUpTrayIcon className="w-4 h-4 text-blue-500" />
                          <span className="text-gray-700 dark:text-gray-200">{user.upload_speed}</span>
                        </div>
                      </td>

                      {/* 下载速度 */}
                      <td className="py-4 px-4">
                        <div className="flex items-center gap-2">
                          <ArrowDownTrayIcon className="w-4 h-4 text-green-500" />
                          <span className="text-gray-700 dark:text-gray-200">{user.download_speed}</span>
                        </div>
                      </td>

                      {/* 总流量 */}
                      <td className="py-4 px-4">
                        <span className="font-semibold text-gray-800 dark:text-gray-100">{user.total_traffic}</span>
                      </td>

                      {/* 流量占比 */}
                      <td className="py-4 px-4">
                        <div className="w-full max-w-[120px]">
                          <div className="flex items-center justify-between mb-1">
                            <span className="text-xs text-gray-500 dark:text-gray-400">{trafficPercent.toFixed(1)}%</span>
                          </div>
                          <div className="w-full progress-bar">
                            <div
                              className={`h-full bg-gradient-to-r ${progressColor} transition-all duration-500`}
                              style={{ width: `${Math.min(trafficPercent, 100)}%` }}
                            />
                          </div>
                        </div>
                      </td>

                      {/* 连接数 */}
                      <td className="py-4 px-4 text-center">
                        <span className="inline-flex items-center justify-center px-3 py-1 rounded-full text-sm font-medium bg-blue-100 dark:bg-blue-900/30 text-blue-800 dark:text-blue-300">
                          {user.active_connections}
                        </span>
                      </td>
                    </tr>
                  )
                })}
              </tbody>
            </table>
          </div>

          {/* 移动端卡片视图 */}
          <div className="md:hidden">
            {users.map((user, index) => {
              const userTraffic = parseTrafficString(user.total_traffic)
              const trafficPercent = totalTraffic > 0 ? (userTraffic / totalTraffic) * 100 : 0
              const progressColor = getProgressColor(trafficPercent)

              return (
                <UserCard
                  key={user.uuid || index}
                  user={user}
                  trafficPercent={trafficPercent}
                  progressColor={progressColor}
                />
              )
            })}
          </div>
        </>
      )}
    </div>
  )
}
