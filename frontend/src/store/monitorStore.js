/**
 * 监控数据状态管理
 * 使用 Zustand 管理全局状态
 * 集成 WebSocket 协调器实现多标签页连接共享
 */
import { create } from 'zustand'
import { createWebSocketClient } from '../api/websocket'
import { createWebSocketCoordinator } from '../api/ws-coordinator'
import * as API from '../api/rest'

const useMonitorStore = create((set, get) => ({
  // ==================== 数据状态 ====================
  uploadSpeed: '0 KB/s',
  downloadSpeed: '0 KB/s',
  totalTraffic: '0 GB',
  uptime: '0s',
  memoryUsage: '0 MB',
  totalMemory: '0 GB',
  activeConnections: 0,
  maxConnections: 300,
  publicIp: '0.0.0.0',
  users: [],

  // 速度历史数据（固定槽位设计）
  speedHistory: [],
  historyDuration: 120,
  // 固定槽数量：每1秒一个槽，共120个槽（120秒）
  historySlots: 120,

  // ==================== 连接状态 ====================
  isConnected: false,
  isPolling: false,
  isLoading: true,
  error: null,

  // ==================== WebSocket 协调器 ====================
  coordinator: null,
  wsClient: null,
  pollingInterval: null,

  // ==================== Actions ====================

  /**
   * 更新监控数据
   */
  updateStats: (stats) => {
    set({
      uploadSpeed: stats.upload_speed || '0 KB/s',
      downloadSpeed: stats.download_speed || '0 KB/s',
      totalTraffic: stats.total_traffic || '0 GB',
      uptime: stats.uptime || '0s',
      memoryUsage: stats.memory_usage || '0 MB',
      totalMemory: stats.total_memory || '0 GB',
      activeConnections: stats.active_connections || 0,
      maxConnections: stats.max_connections || 300,
      publicIp: stats.public_ip || '0.0.0.0',
      users: stats.users || [],
      isLoading: false,
      error: null,
    })
  },

  /**
   * 更新速度历史（固定槽位设计）
   * 用于 API 轮询模式，避免重复数据
   */
  updateHistory: (historyData) => {
    set((state) => {
      // 后端返回的历史数据
      const backendHistory = historyData.history || []

      // 固定槽位设计：只保留最后的60个数据点
      const fixedHistory = backendHistory.slice(-state.historySlots)

      return {
        speedHistory: fixedHistory,
        historyDuration: historyData.duration_seconds || 120,
      }
    })
  },

  /**
   * 追加速度历史数据点（固定槽位设计）
   * @param {string} uploadSpeed - 上传速度（如 "1.23 MB/s"）
   * @param {string} downloadSpeed - 下载速度（如 "2.34 MB/s"）
   * @param {string} timestamp - Unix 时间戳（秒），可选，如果不提供则使用当前时间
   *
   * 设计原理：
   * - 使用固定槽数量（120个槽，每个槽代表1秒）
   * - 新数据追加到末尾，移除最旧的数据
   * - 图表显示固定的120个点，实现平滑滑动效果
   */
  appendHistoryPoint: (uploadSpeed, downloadSpeed, timestamp) => {
    set((state) => {
      // 使用后端提供的时间戳（秒），如果没有则使用当前时间（转换为秒）
      const unixTimestamp = timestamp || Math.floor(Date.now() / 1000)

      const newPoint = {
        timestamp: unixTimestamp.toString(),
        upload_speed: uploadSpeed,
        download_speed: downloadSpeed,
      }

      // 避免重复：检查最后一个点的时间戳
      const lastPoint = state.speedHistory[state.speedHistory.length - 1]
      if (lastPoint && lastPoint.timestamp === newPoint.timestamp) {
        // 时间戳相同，不追加（避免重复）
        return {}
      }

      // 固定槽位设计：保持固定数量的数据点
      // 新数据追加到末尾，移除最旧的数据
      const newHistory = [...state.speedHistory, newPoint].slice(-state.historySlots)

      return { speedHistory: newHistory }
    })
  },

  /**
   * 连接 WebSocket（使用协调器实现多标签页共享）
   */
  connect: () => {
    const coordinator = createWebSocketCoordinator()

    // 数据处理回调
    const handleData = (message) => {
      if (message.type === 'history') {
        // 初始化历史数据（连接建立时的完整历史）
        get().updateHistory(message.payload)
      } else if (message.type === 'stats') {
        // 更新实时数据
        get().updateStats(message.payload)

        // 追加当前速度到历史（仅 WebSocket 模式需要）
        const { upload_speed, download_speed, timestamp } = message.payload
        get().appendHistoryPoint(upload_speed, download_speed, timestamp)
      }
    }

    // 初始化协调器
    coordinator.init(handleData)

    // 检查是否成为主连接
    setTimeout(() => {
      const status = coordinator.getStatus()

      if (status.isMaster) {
        // 主连接：使用 WebSocket
        console.log('[Monitor] 主连接模式，使用 WebSocket')
        set({
          coordinator,
          isConnected: status.hasWebSocket,
          isPolling: false,
        })
      } else {
        // 从连接：直接使用轮询
        console.log('[Monitor] 从连接模式，使用 API 轮询')
        get().startPolling()
        set({
          coordinator,
          isConnected: false,
          isPolling: true,
        })
      }
    }, 1000) // 等待选举完成
  },

  /**
   * 断开连接
   */
  disconnect: () => {
    const { coordinator, wsClient, pollingInterval } = get()

    // 销毁协调器
    if (coordinator) {
      coordinator.destroy()
    }

    // 兼容旧的 wsClient
    if (wsClient) {
      wsClient.disconnect()
    }

    if (pollingInterval) {
      clearInterval(pollingInterval)
    }

    set({
      coordinator: null,
      wsClient: null,
      isConnected: false,
      isPolling: false,
      pollingInterval: null,
    })
  },

  /**
   * 启动 API 轮询
   */
  startPolling: async () => {
    const { pollingInterval, fetchAllData } = get()

    if (pollingInterval) {
      return // 已经在轮询
    }

    console.log('[Monitor] 启动 API 轮询')

    // 立即获取一次数据
    await fetchAllData()

    // 每秒轮询
    const interval = setInterval(async () => {
      await fetchAllData()
    }, 1000)

    set({
      pollingInterval: interval,
      isPolling: true,
      isConnected: false,
    })
  },

  /**
   * 获取所有数据
   */
  fetchAllData: async () => {
    try {
      const [stats, history] = await Promise.all([
        API.fetchStats(),
        API.fetchSpeedHistory(),
      ])

      get().updateStats(stats)
      get().updateHistory(history)

      set({
        isLoading: false,
        error: null,
      })
    } catch (error) {
      console.error('[Monitor] 获取数据失败:', error)
      set({
        error: error.message,
        isLoading: false,
      })
    }
  },

  /**
   * 获取内存使用百分比
   */
  getMemoryPercent: () => {
    const { memoryUsage, totalMemory } = get()

    const parseMemory = (str) => {
      const match = str.match(/([\d.]+)\s*(\w+)/)
      if (!match) return 0

      const value = parseFloat(match[1])
      const unit = match[2]

      switch (unit) {
        case 'MB':
          return value
        case 'GB':
          return value * 1024
        case 'TB':
          return value * 1024 * 1024
        default:
          return value
      }
    }

    const usage = parseMemory(memoryUsage)
    const total = parseMemory(totalMemory)

    if (total === 0) return 0
    return Math.round((usage / total) * 100)
  },
}))

export default useMonitorStore
