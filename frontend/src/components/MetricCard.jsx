/**
 * 基础指标卡片组件（增强版）
 */
import { clsx } from 'clsx'
import AnimatedNumber from './AnimatedNumber'

export default function MetricCard({
  title,
  value,
  unit = '',
  icon: Icon,
  color = 'blue',
  trend = null,
  className = '',
  animate = true,
}) {
  const colorClasses = {
    blue: {
      gradient: 'from-blue-500 to-cyan-400',
      shadow: 'shadow-blue-500/30',
      text: 'text-blue-600 dark:text-blue-400',
    },
    green: {
      gradient: 'from-green-500 to-emerald-400',
      shadow: 'shadow-green-500/30',
      text: 'text-green-600 dark:text-green-400',
    },
    purple: {
      gradient: 'from-purple-500 to-pink-400',
      shadow: 'shadow-purple-500/30',
      text: 'text-purple-600 dark:text-purple-400',
    },
    orange: {
      gradient: 'from-orange-500 to-amber-400',
      shadow: 'shadow-orange-500/30',
      text: 'text-orange-600 dark:text-orange-400',
    },
    red: {
      gradient: 'from-red-500 to-rose-400',
      shadow: 'shadow-red-500/30',
      text: 'text-red-600 dark:text-red-400',
    },
  }

  const colorScheme = colorClasses[color] || colorClasses.blue

  // 将数值转换为数字（用于动画）
  const numericValue = typeof value === 'number' ? value : parseFloat(value) || 0

  return (
    <div className={clsx('glass-card p-6 btn-hover', className)}>
      <div className="flex items-start justify-between mb-4">
        <div className="flex-1">
          <p className="metric-label">{title}</p>
          <div className="flex items-baseline gap-2 mt-2">
            {animate ? (
              <>
                <span className="metric-value">
                  <AnimatedNumber value={numericValue} decimals={numericValue < 10 ? 2 : 0} />
                </span>
                {unit && (
                  <span className="text-sm text-gray-500 dark:text-gray-400">{unit}</span>
                )}
              </>
            ) : (
              <>
                <span className="metric-value">{value}</span>
                {unit && (
                  <span className="text-sm text-gray-500 dark:text-gray-400">{unit}</span>
                )}
              </>
            )}
          </div>
        </div>
        {Icon && (
          <div
            className={clsx(
              'p-3 rounded-xl bg-gradient-to-br shadow-lg transition-transform duration-300 hover:scale-110',
              colorScheme.gradient,
              colorScheme.shadow
            )}
          >
            <Icon className="w-6 h-6 text-white" />
          </div>
        )}
      </div>

      {trend && (
        <div className="flex items-center gap-2 mt-2">
          <span
            className={clsx(
              'text-sm font-medium px-2 py-1 rounded-lg transition-all duration-300',
              trend === 'up'
                ? 'text-green-600 dark:text-green-400 bg-green-50 dark:bg-green-900/20'
                : 'text-red-600 dark:text-red-400 bg-red-50 dark:bg-red-900/20'
            )}
          >
            {trend === 'up' ? '↑ 上升' : '↓ 下降'}
          </span>
        </div>
      )}
    </div>
  )
}
