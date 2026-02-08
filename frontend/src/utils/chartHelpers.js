/**
 * 图表辅助函数
 * 用于 Victory 图表库的样式和格式化
 */

/**
 * 格式化速度显示（KB/s 或 MB/s）
 * @param {number} value - 速度值（MB/s）
 * @returns {string} 格式化后的速度字符串
 */
export function formatSpeed(value) {
  if (value < 0.001) return '0 KB/s'
  if (value < 1) return `${(value * 1024).toFixed(0)} KB/s`
  if (value < 10) return `${value.toFixed(2)} MB/s`
  return `${value.toFixed(1)} MB/s`
}

/**
 * 格式化相对时间
 * @param {number} seconds - 秒数
 * @returns {string} 格式化后的相对时间字符串
 */
export function formatRelativeTime(seconds) {
  if (seconds === 0) return '现在'
  if (seconds < 60) return `${seconds}秒前`

  const mins = Math.floor(seconds / 60)
  const secs = seconds % 60
  return secs === 0 ? `${mins}分钟前` : `${mins}分${secs}秒前`
}

/**
 * 计算图表 Y 轴的合适最大值
 * @param {number[]} values - 数据值数组
 * @returns {number} 合适的最大值
 */
export function calculateYAxisMax(values) {
  if (!values || values.length === 0) return 1

  const max = Math.max(...values)
  
  if (max < 0.1) return 0.1
  if (max < 1) return Math.ceil(max * 10) / 10
  if (max < 10) return Math.ceil(max)
  return Math.ceil(max / 10) * 10
}

/**
 * 生成图表 X 轴刻度值
 * @param {number} dataLength - 数据点数量
 * @param {number} maxTicks - 最大刻度数
 * @returns {number[]} 刻度索引数组
 */
export function generateXAxisTicks(dataLength, maxTicks = 5) {
  if (dataLength === 0) return []
  if (dataLength <= maxTicks) return Array.from({ length: dataLength }, (_, i) => i)

  const step = Math.floor(dataLength / (maxTicks - 1))
  const ticks = []
  
  for (let i = 0; i < maxTicks - 1; i++) {
    ticks.push(i * step)
  }
  ticks.push(dataLength - 1)
  
  return ticks
}

/**
 * 图表主题配置
 */
export const chartTheme = {
  colors: {
    upload: '#3b82f6',
    download: '#10b981',
    grid: 'hsl(var(--border))',
    text: 'hsl(var(--foreground))',
    mutedText: 'hsl(var(--muted-foreground))'
  },
  gradients: {
    upload: {
      start: { color: '#3b82f6', opacity: 0.4 },
      end: { color: '#3b82f6', opacity: 0.05 }
    },
    download: {
      start: { color: '#10b981', opacity: 0.4 },
      end: { color: '#10b981', opacity: 0.05 }
    }
  }
}
