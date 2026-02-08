/**
 * 骨架屏加载组件
 * 用于数据加载时的占位显示
 */
import { clsx } from 'clsx'

export default function LoadingSkeleton({ className = '' }) {
  return (
    <div className={clsx('animate-pulse', className)}>
      <div className="h-4 bg-gray-200 dark:bg-gray-700 rounded w-3/4 mb-2"></div>
      <div className="h-4 bg-gray-200 dark:bg-gray-700 rounded w-1/2"></div>
    </div>
  )
}

/**
 * 卡片骨架屏
 */
export function CardSkeleton({ className = '' }) {
  return (
    <div className={clsx('glass-card p-6', className)}>
      <div className="animate-pulse">
        <div className="h-5 bg-gray-200 dark:bg-gray-700 rounded w-1/3 mb-4"></div>
        <div className="h-8 bg-gray-200 dark:bg-gray-700 rounded w-2/3 mb-2"></div>
        <div className="h-4 bg-gray-200 dark:bg-gray-700 rounded w-1/4"></div>
      </div>
    </div>
  )
}

/**
 * 圆形图标骨架屏
 */
export function IconSkeleton({ className = '' }) {
  return (
    <div className={clsx('animate-pulse rounded-xl bg-gray-200 dark:bg-gray-700', className)} />
  )
}

/**
 * 表格行骨架屏
 */
export function TableRowSkeleton({ rows = 3 }) {
  return (
    <>
      {Array.from({ length: rows }).map((_, i) => (
        <tr key={i} className="border-b border-gray-100 dark:border-gray-700">
          <td className="py-4 px-4">
            <div className="animate-pulse">
              <div className="h-4 bg-gray-200 dark:bg-gray-700 rounded w-24 mb-2"></div>
              <div className="h-3 bg-gray-200 dark:bg-gray-700 rounded w-16"></div>
            </div>
          </td>
          <td className="py-4 px-4">
            <LoadingSkeleton />
          </td>
          <td className="py-4 px-4">
            <LoadingSkeleton />
          </td>
          <td className="py-4 px-4">
            <LoadingSkeleton />
          </td>
          <td className="py-4 px-4">
            <div className="animate-pulse">
              <div className="h-2 bg-gray-200 dark:bg-gray-700 rounded w-full max-w-[120px]"></div>
            </div>
          </td>
          <td className="py-4 px-4 text-center">
            <div className="animate-pulse inline-flex items-center justify-center px-3 py-1 rounded-full bg-gray-200 dark:bg-gray-700 w-12 h-6"></div>
          </td>
        </tr>
      ))}
    </>
  )
}

/**
 * 图表骨架屏
 */
export function ChartSkeleton({ className = '' }) {
  return (
    <div className={clsx('glass-card p-6 h-full', className)}>
      <div className="animate-pulse">
        <div className="h-6 bg-gray-200 dark:bg-gray-700 rounded w-1/3 mb-2"></div>
        <div className="h-4 bg-gray-200 dark:bg-gray-700 rounded w-1/4 mb-6"></div>
        <div className="chart-container bg-gray-100 dark:bg-gray-800 rounded-xl flex items-center justify-center">
          <div className="text-center">
            <div className="inline-block w-12 h-12 border-4 border-gray-200 dark:border-gray-700 border-t-blue-500 rounded-full animate-spin"></div>
            <p className="mt-4 text-sm text-gray-500 dark:text-gray-400">加载中...</p>
          </div>
        </div>
      </div>
    </div>
  )
}
