/**
 * 数据格式化工具函数
 */

/**
 * 解析速度字符串为数值 (例如: "1.23 MB/s" -> 1.23)
 * @param {string} speedStr - 速度字符串
 * @returns {number} - 速度数值 (MB/s)
 */
export function parseSpeedString(speedStr) {
  if (!speedStr || typeof speedStr !== 'string') return 0

  const parts = speedStr.split(' ')
  if (parts.length < 2) return 0

  const value = parseFloat(parts[0])
  const unit = parts[1]

  // 转换为 MB/s
  switch (unit) {
    case 'KB/s':
      return value / 1024
    case 'MB/s':
      return value
    case 'GB/s':
      return value * 1024
    default:
      return value
  }
}

/**
 * 解析流量字符串为字节数 (例如: "1.23 GB" -> 字节数)
 * @param {string} trafficStr - 流量字符串
 * @returns {number} - 字节数
 */
export function parseTrafficString(trafficStr) {
  if (!trafficStr || typeof trafficStr !== 'string') return 0

  const parts = trafficStr.split(' ')
  if (parts.length < 2) return 0

  const value = parseFloat(parts[0])
  const unit = parts[1]

  // 转换为字节
  switch (unit) {
    case 'KB':
      return value * 1024
    case 'MB':
      return value * 1024 * 1024
    case 'GB':
      return value * 1024 * 1024 * 1024
    case 'TB':
      return value * 1024 * 1024 * 1024 * 1024
    default:
      return value
  }
}

/**
 * 解析内存字符串 (例如: "45.67 MB" -> 字节数)
 * @param {string} memoryStr - 内存字符串
 * @returns {number} - 字节数
 */
export function parseMemoryString(memoryStr) {
  return parseTrafficString(memoryStr)
}

/**
 * 解析运行时长字符串 (例如: "2d 5h 30m 15s" -> 秒数)
 * @param {string} uptimeStr - 运行时长字符串
 * @returns {number} - 秒数
 */
export function parseUptimeString(uptimeStr) {
  if (!uptimeStr || typeof uptimeStr !== 'string') return 0

  let totalSeconds = 0
  const parts = uptimeStr.split(' ')

  for (let i = 0; i < parts.length; i++) {
    const value = parseInt(parts[i])
    const unit = parts[i + 1]

    if (isNaN(value)) continue

    switch (unit) {
      case 'd':
        totalSeconds += value * 86400
        break
      case 'h':
        totalSeconds += value * 3600
        break
      case 'm':
        totalSeconds += value * 60
        break
      case 's':
        totalSeconds += value
        break
    }
  }

  return totalSeconds
}

/**
 * 计算内存使用百分比
 * @param {string} usageStr - 内存使用量字符串
 * @param {string} totalStr - 总内存字符串
 * @returns {number} - 百分比
 */
export function calculateMemoryPercent(usageStr, totalStr) {
  const usage = parseMemoryString(usageStr)
  const total = parseMemoryString(totalStr)

  if (total === 0) return 0
  return Math.round((usage / total) * 100)
}

/**
 * 格式化进度条颜色类名
 * @param {number} percent - 百分比值
 * @returns {string} - Tailwind 颜色类名
 */
export function getProgressColor(percent) {
  if (percent < 50) return 'from-green-400 to-green-500'
  if (percent < 75) return 'from-yellow-400 to-yellow-500'
  if (percent < 90) return 'from-orange-400 to-orange-500'
  return 'from-red-400 to-red-500'
}

/**
 * 截断 UUID 显示
 * @param {string} uuid - 完整 UUID
 * @returns {string} - 截断后的 UUID
 */
export function truncateUUID(uuid) {
  if (!uuid) return 'N/A'
  if (uuid.length <= 16) return uuid
  return `${uuid.slice(0, 8)}...${uuid.slice(-8)}`
}

/**
 * 格式化时间戳为相对时间
 * @param {number} timestamp - 秒数时间戳
 * @returns {string} - 相对时间字符串
 */
export function formatRelativeTime(timestamp) {
  if (!timestamp) return '0秒前'

  const seconds = parseInt(timestamp)
  if (seconds < 60) return `${seconds}秒前`

  const minutes = Math.floor(seconds / 60)
  if (minutes < 60) return `${minutes}分钟前`

  const hours = Math.floor(minutes / 60)
  if (hours < 24) return `${hours}小时前`

  const days = Math.floor(hours / 24)
  return `${days}天前`
}
