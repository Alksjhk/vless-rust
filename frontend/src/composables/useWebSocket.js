import { ref, shallowRef, onMounted, onUnmounted } from 'vue'

// 默认配置
const DEFAULT_CONFIG = {
  speed_history_duration: 60,
  broadcast_interval: 1,
  websocket_max_connections: 100,
  websocket_heartbeat_timeout: 60,
  vless_max_connections: 100
}

// 单例状态
const state = {
  stats: ref({
    upload_speed: '--',
    download_speed: '--',
    total_traffic: '--',
    uptime: '--',
    memory_usage: '--',
    total_memory: '--',
    active_connections: 0,
    max_connections: 100,
    start_time: null
  }),
  loading: ref(true),
  error: ref(null),
  connected: ref(false),
  uploadPeak: shallowRef(0),
  downloadPeak: shallowRef(0),
  trafficHistory: shallowRef([]),
  uploadHistory: shallowRef([]),
  downloadHistory: shallowRef([]),
  isDataSynced: ref(false),
  useFallback: false,
  clients: 0,
  config: null
}

const DATA_POINTS = 60
const STORAGE_KEY = 'vless_traffic_history'
const TWO_MINUTES = 2 * 60 * 1000

let ws = null
let fallbackInterval = null
let isInitialized = false

// 获取配置
const fetchConfig = async () => {
  try {
    const response = await fetch('/api/config')
    if (response.status === 404) return DEFAULT_CONFIG
    return await response.json()
  } catch {
    return DEFAULT_CONFIG
  }
}

// 工具函数
const isValidHistoryData = (data, expectedLength) => {
  if (!Array.isArray(data)) return false
  if (data.length !== expectedLength) return false
  if (!data.timestamp) return false
  if (Date.now() - data.timestamp > TWO_MINUTES) return false
  return true
}

const loadHistoryFromStorage = (dataPoints) => {
  try {
    const stored = sessionStorage.getItem(STORAGE_KEY)
    if (stored) {
      const data = JSON.parse(stored)
      if (isValidHistoryData(data, dataPoints)) {
        return {
          upload: data.upload || Array(dataPoints).fill(0),
          download: data.download || Array(dataPoints).fill(0),
          traffic: data.traffic || Array(dataPoints).fill(0)
        }
      }
    }
  } catch (err) {
    console.error('Failed to load history from storage:', err)
  }
  return null
}

const saveHistoryToStorage = (upload, download, traffic) => {
  try {
    const data = {
      upload,
      download,
      traffic,
      timestamp: Date.now()
    }
    sessionStorage.setItem(STORAGE_KEY, JSON.stringify(data))
  } catch (err) {
    console.error('Failed to save history to storage:', err)
  }
}

const createEmptyHistory = (dataPoints) => Array(dataPoints).fill(0)

const initializeHistory = (dataPoints = 60) => {
  const storedData = loadHistoryFromStorage(dataPoints)
  if (storedData) {
    state.uploadHistory.value = [...storedData.upload]
    state.downloadHistory.value = [...storedData.download]
    state.trafficHistory.value = [...storedData.traffic]
    state.isDataSynced.value = true
  } else {
    state.uploadHistory.value = createEmptyHistory(dataPoints)
    state.downloadHistory.value = createEmptyHistory(dataPoints)
    state.trafficHistory.value = createEmptyHistory(dataPoints)
    state.isDataSynced.value = false
  }
}

const parseSize = (sizeStr) => {
  const match = sizeStr.match(/^([\d.]+)\s*(B|KB|MB|GB|TB)(\/s)?$/)
  if (!match) return 0

  const value = parseFloat(match[1])
  const unit = match[2]

  const multipliers = {
    'B': 1,
    'KB': 1024,
    'MB': 1024 * 1024,
    'GB': 1024 * 1024 * 1024,
    'TB': 1024 * 1024 * 1024 * 1024
  }
  return value * (multipliers[unit] || 1)
}

const parseSpeed = (speedStr) => parseSize(speedStr)

const formatBytes = (bytes) => {
  if (bytes === 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i]
}

const updateStats = (data) => {
  state.stats.value = data

  const uploadSpeed = parseSpeed(data.upload_speed)
  const downloadSpeed = parseSpeed(data.download_speed)

  if (uploadSpeed > state.uploadPeak.value) {
    state.uploadPeak.value = uploadSpeed
  }
  if (downloadSpeed > state.downloadPeak.value) {
    state.downloadPeak.value = downloadSpeed
  }

  const totalSpeed = uploadSpeed + downloadSpeed
  const dataPoints = state.uploadHistory.value.length
  const newTrafficHistory = [...state.trafficHistory.value.slice(1), totalSpeed]
  const newUploadHistory = [...state.uploadHistory.value.slice(1), uploadSpeed]
  const newDownloadHistory = [...state.downloadHistory.value.slice(1), downloadSpeed]

  state.trafficHistory.value = newTrafficHistory
  state.uploadHistory.value = newUploadHistory
  state.downloadHistory.value = newDownloadHistory

  saveHistoryToStorage(newUploadHistory, newDownloadHistory, newTrafficHistory)

  if (!state.isDataSynced.value) {
    state.isDataSynced.value = true
  }

  state.loading.value = false
}

// API 降级逻辑
const fetchStats = async () => {
  state.error.value = null

  try {
    const response = await fetch('/api/stats')
    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`)
    }

    const data = await response.json()
    updateStats(data)
  } catch (err) {
    state.error.value = err.message
    state.loading.value = false
    console.error('Failed to fetch stats:', err)
  }
}

const startFallback = () => {
  if (fallbackInterval) return

  console.log('Starting API fallback polling')
  state.useFallback = true
  fetchStats()
  fallbackInterval = setInterval(fetchStats, 1000)
}

const stopFallback = () => {
  if (fallbackInterval) {
    clearInterval(fallbackInterval)
    fallbackInterval = null
    console.log('Stopped API fallback polling')
  }
  state.useFallback = false
}

// WebSocket 连接
const connect = () => {
  const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:'
  const host = window.location.host
  const wsUrl = `${protocol}//${host}/api/ws`

  stopFallback()

  ws = new WebSocket(wsUrl)

  ws.onopen = () => {
    console.log('WebSocket connected')
    state.connected.value = true
    state.error.value = null
    state.useFallback = false
  }

  ws.onmessage = (event) => {
    try {
      const msg = JSON.parse(event.data)

      if (msg.type === 'stats') {
        updateStats(msg.payload)
      } else if (msg.type === 'history') {
        if (msg.payload.history && msg.payload.history.length > 0) {
          const newUploadHistory = []
          const newDownloadHistory = []
          const newTrafficHistory = []

          msg.payload.history.forEach(item => {
            const upload = parseSpeed(item.upload_speed)
            const download = parseSpeed(item.download_speed)
            newUploadHistory.push(upload)
            newDownloadHistory.push(download)
            newTrafficHistory.push(upload + download)
          })

          // 使用配置中的数据点数量
          const dataPoints = state.config ? Math.floor(state.config.speed_history_duration / state.config.broadcast_interval) : 60

          if (newTrafficHistory.length < dataPoints) {
            const padding = dataPoints - newTrafficHistory.length
            for (let i = 0; i < padding; i++) {
              newUploadHistory.unshift(0)
              newDownloadHistory.unshift(0)
              newTrafficHistory.unshift(0)
            }
          }

          state.uploadHistory.value = newUploadHistory.slice(-dataPoints)
          state.downloadHistory.value = newDownloadHistory.slice(-dataPoints)
          state.trafficHistory.value = newTrafficHistory.slice(-dataPoints)
          state.isDataSynced.value = true
          saveHistoryToStorage(
            state.uploadHistory.value,
            state.downloadHistory.value,
            state.trafficHistory.value
          )
        }
      }
    } catch (err) {
      console.error('Failed to parse WebSocket message:', err)
    }
  }

  ws.onerror = (event) => {
    console.error('WebSocket error:', event)
    state.error.value = '连接错误'
  }

  ws.onclose = (event) => {
    console.log('WebSocket closed:', event.code, event.reason)
    state.connected.value = false
    state.loading.value = false

    if (event.code !== 1000 && !state.useFallback) {
      state.error.value = 'WebSocket 连接已断开，正在使用 API 轮询'
      // 启动 API 降级
      startFallback()
    }
  }
}

const reconnect = () => {
  stopFallback()
  if (ws) {
    ws.close()
  }
  state.error.value = null
  state.loading.value = true
  connect()
}

const getUploadProgress = () => {
  const uploadSpeed = parseSpeed(state.stats.value.upload_speed)
  const maxSpeed = 10 * 1024 * 1024
  return Math.min((uploadSpeed / maxSpeed) * 100, 100)
}

const getDownloadProgress = () => {
  const downloadSpeed = parseSpeed(state.stats.value.download_speed)
  const maxSpeed = 10 * 1024 * 1024
  return Math.min((downloadSpeed / maxSpeed) * 100, 100)
}

const getMemoryProgress = () => {
  const memoryBytes = parseSize(state.stats.value.memory_usage)
  const totalMemory = parseSize(state.stats.value.total_memory)
  return Math.min((memoryBytes / totalMemory) * 100, 100)
}

const getConnectionsProgress = () => {
  return Math.min((state.stats.value.active_connections / state.stats.value.max_connections) * 100, 100)
}

const calculateAdjustedMaxSpeed = (maxSpeed, minStep) => {
  if (maxSpeed <= minStep) {
    return minStep
  }
  const power = Math.pow(10, Math.floor(Math.log10(maxSpeed)))
  const mantissa = maxSpeed / power

  if (mantissa <= 1) return power
  if (mantissa <= 2) return 2 * power
  if (mantissa <= 5) return 5 * power
  return 10 * power
}

const getChartData = () => {
  const upload = state.uploadHistory.value
  const download = state.downloadHistory.value
  const traffic = state.trafficHistory.value

  if (upload.length === 0) {
    initializeHistory()
    return []
  }

  const safeUpload = state.uploadHistory.value
  const safeDownload = state.downloadHistory.value
  const safeTraffic = state.trafficHistory.value

  const maxSpeed = Math.max(...safeTraffic, ...safeUpload, ...safeDownload, 1)
  const minStep = 100 * 1024
  const adjustedMaxSpeed = calculateAdjustedMaxSpeed(maxSpeed, minStep)

  const len = safeTraffic.length
  return safeTraffic.map((speed, index) => {
    const upVal = safeUpload[index] || 0
    const downVal = safeDownload[index] || 0
    const adjustedMax = adjustedMaxSpeed
    return {
      index,
      total: speed,
      upload: upVal,
      download: downVal,
      maxSpeed,
      adjustedMaxSpeed: adjustedMax,
      totalHeight: Math.max((speed / adjustedMax) * 100, 0.5),
      uploadHeight: Math.max((upVal / adjustedMax) * 100, 0.5),
      downloadHeight: Math.max((downVal / adjustedMax) * 100, 0.5)
    }
  })
}

// 初始化
const init = async () => {
  if (!isInitialized) {
    isInitialized = true

    // 获取配置
    const config = await fetchConfig()
    state.config = config

    // 根据配置计算数据点数量
    const DATA_POINTS = Math.floor(config.speed_history_duration / config.broadcast_interval)

    // 更新 max_connections
    state.stats.value.max_connections = config.vless_max_connections

    initializeHistory(DATA_POINTS)
    connect()
  }
}

// 清理
const cleanup = () => {
  state.clients--
  if (state.clients <= 0) {
    state.clients = 0
    stopFallback()
    if (ws) {
      ws.close(1000, 'All components unmounted')
      ws = null
    }
    isInitialized = false
  }
}

export function useWebSocket() {
  state.clients++

  onMounted(() => {
    init()
  })

  onUnmounted(() => {
    cleanup()
  })

  return {
    stats: state.stats,
    loading: state.loading,
    error: state.error,
    connected: state.connected,
    uploadPeak: state.uploadPeak,
    downloadPeak: state.downloadPeak,
    trafficHistory: state.trafficHistory,
    isDataSynced: state.isDataSynced,
    reconnect,
    formatBytes,
    getUploadProgress,
    getDownloadProgress,
    getMemoryProgress,
    getConnectionsProgress,
    getChartData
  }
}
