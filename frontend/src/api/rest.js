/**
 * REST API 封装
 * 用于 WebSocket 失败时的降级方案
 */

const API_BASE_URL = import.meta.env.PROD ? '' : '/api'

/**
 * 获取实时监控数据
 * @returns {Promise<Object>} 监控数据
 */
export async function fetchStats() {
  const response = await fetch(`${API_BASE_URL}/stats`)

  if (!response.ok) {
    throw new Error(`获取监控数据失败: ${response.status}`)
  }

  return response.json()
}

/**
 * 获取速度历史数据
 * @returns {Promise<Object>} 历史数据
 */
export async function fetchSpeedHistory() {
  const response = await fetch(`${API_BASE_URL}/speed-history`)

  if (!response.ok) {
    throw new Error(`获取历史数据失败: ${response.status}`)
  }

  return response.json()
}

/**
 * 获取用户统计
 * @returns {Promise<Array>} 用户统计数据
 */
export async function fetchUserStats() {
  const response = await fetch(`${API_BASE_URL}/user-stats`)

  if (!response.ok) {
    throw new Error(`获取用户统计失败: ${response.status}`)
  }

  return response.json()
}

/**
 * 获取监控配置
 * @returns {Promise<Object>} 配置数据
 */
export async function fetchConfig() {
  const response = await fetch(`${API_BASE_URL}/config`)

  if (!response.ok) {
    throw new Error(`获取配置失败: ${response.status}`)
  }

  return response.json()
}
