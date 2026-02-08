/**
 * WebSocket 连接管理
 * 实现智能降级策略
 */

class WebSocketClient {
  constructor(url) {
    this.url = url
    this.ws = null
    this.reconnectAttempts = 0
    this.maxReconnectAttempts = 3
    this.reconnectDelay = 1000
    this.maxReconnectDelay = 30000
    this.messageHandlers = new Set()
    this.isConnected = false
  }

  /**
   * 连接 WebSocket
   */
  connect() {
    try {
      this.ws = new WebSocket(this.url)

      this.ws.onopen = () => {
        console.log('[WebSocket] 已连接')
        this.isConnected = true
        this.reconnectAttempts = 0
        this.reconnectDelay = 1000
      }

      this.ws.onmessage = (event) => {
        try {
          const message = JSON.parse(event.data)
          this.notifyHandlers(message)
        } catch (error) {
          console.error('[WebSocket] 消息解析失败:', error)
        }
      }

      this.ws.onerror = (error) => {
        console.error('[WebSocket] 错误:', error)
      }

      this.ws.onclose = (event) => {
        console.log('[WebSocket] 已关闭:', event.code, event.reason)
        this.isConnected = false
        this.ws = null

        // 尝试重连
        if (this.reconnectAttempts < this.maxReconnectAttempts) {
          this.scheduleReconnect()
        }
      }
    } catch (error) {
      console.error('[WebSocket] 连接失败:', error)
      return false
    }

    return true
  }

  /**
   * 安排重连
   */
  scheduleReconnect() {
    this.reconnectAttempts++
    const delay = Math.min(
      this.reconnectDelay * Math.pow(2, this.reconnectAttempts - 1),
      this.maxReconnectDelay
    )

    console.log(`[WebSocket] ${delay}ms 后尝试第 ${this.reconnectAttempts} 次重连...`)

    setTimeout(() => {
      this.connect()
    }, delay)
  }

  /**
   * 断开连接
   */
  disconnect() {
    if (this.ws) {
      this.ws.close()
      this.ws = null
    }
    this.isConnected = false
    this.reconnectAttempts = this.maxReconnectAttempts // 停止重连
  }

  /**
   * 添加消息处理器
   * @param {Function} handler - 消息处理函数
   */
  onMessage(handler) {
    this.messageHandlers.add(handler)
  }

  /**
   * 移除消息处理器
   * @param {Function} handler - 消息处理函数
   */
  offMessage(handler) {
    this.messageHandlers.delete(handler)
  }

  /**
   * 通知所有处理器
   * @param {Object} message - 消息对象
   */
  notifyHandlers(message) {
    this.messageHandlers.forEach(handler => {
      try {
        handler(message)
      } catch (error) {
        console.error('[WebSocket] 处理器错误:', error)
      }
    })
  }

  /**
   * 发送消息
   * @param {Object} data - 消息数据
   */
  send(data) {
    if (this.ws && this.isConnected) {
      this.ws.send(JSON.stringify(data))
    }
  }
}

/**
 * 创建 WebSocket 客户端实例
 * @returns {WebSocketClient} WebSocket 客户端
 */
export function createWebSocketClient() {
  // 检测协议
  const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:'
  const host = import.meta.env.PROD ? window.location.host : 'localhost:8443'
  const url = `${protocol}//${host}/api/ws`

  return new WebSocketClient(url)
}

export default WebSocketClient
