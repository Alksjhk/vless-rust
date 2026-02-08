/**
 * WebSocket 多标签页协调器
 * 使用 BroadcastChannel API 确保同一用户只有少量活跃的 WebSocket 连接
 */

import { createWebSocketClient } from './websocket'

const MESSAGE_TYPES = {
  ELECT_MASTER: 'elect_master',
  IAM_MASTER: 'iam_master',
  MASTER_HEARTBEAT: 'master_heartbeat',
  DATA_BROADCAST: 'data_broadcast',
}

class WebSocketCoordinator {
  constructor() {
    this.channel = null
    this.isMaster = false
    this.masterTabId = null
    this.myTabId = this.generateTabId()
    this.wsClient = null
    this.dataHandler = null
    this.heartbeatTimer = null
    this.electionTimer = null
    this.heartbeatTimeout = null
    this.isActive = false
  }

  /**
   * 生成唯一标签页ID
   */
  generateTabId() {
    return `tab_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`
  }

  /**
   * 初始化协调器
   * @param {Function} onData - 数据处理回调
   */
  init(onData) {
    this.dataHandler = onData

    try {
      // 检查浏览器是否支持 BroadcastChannel
      if (typeof BroadcastChannel === 'undefined') {
        console.warn('[WS Coordinator] BroadcastChannel not supported, falling back to independent connection')
        this.becomeMaster()
        return
      }

      this.channel = new BroadcastChannel('ws-coordinator')
      this.channel.onmessage = (event) => this.handleChannelMessage(event.data)

      // 启动主连接选举
      this.startElection()
    } catch (error) {
      console.error('[WS Coordinator] Initialization failed:', error)
      this.becomeMaster()
    }
  }

  /**
   * 启动主连接选举
   */
  startElection() {
    this.channel.postMessage({
      type: MESSAGE_TYPES.ELECT_MASTER,
      tabId: this.myTabId
    })

    // 等待500ms，如果没有其他主连接，则成为主连接
    this.electionTimer = setTimeout(() => {
      if (!this.masterTabId) {
        this.becomeMaster()
      }
    }, 500)
  }

  /**
   * 成为主连接
   */
  becomeMaster() {
    this.isMaster = true
    this.masterTabId = this.myTabId
    this.isActive = true

    console.log('[WS Coordinator] Became master:', this.myTabId)

    // 通知其他标签页
    if (this.channel) {
      this.channel.postMessage({
        type: MESSAGE_TYPES.IAM_MASTER,
        tabId: this.myTabId
      })
    }

    // 启动心跳
    this.startHeartbeat()

    // 创建 WebSocket 连接
    this.connectWebSocket()
  }

  /**
   * 成为从连接
   */
  becomeSlave(masterTabId) {
    this.isMaster = false
    this.masterTabId = masterTabId
    this.isActive = true

    console.log('[WS Coordinator] Became slave, master:', masterTabId)

    // 清理选举定时器
    if (this.electionTimer) {
      clearTimeout(this.electionTimer)
      this.electionTimer = null
    }

    // 启动心跳超时检测
    this.startHeartbeatTimeout()

    // 从连接不创建 WS，直接降级到轮询
    this.startPolling()
  }

  /**
   * 启动心跳发送（仅主连接）
   */
  startHeartbeat() {
    if (this.heartbeatTimer) {
      clearInterval(this.heartbeatTimer)
    }

    this.heartbeatTimer = setInterval(() => {
      if (this.channel && this.isMaster) {
        this.channel.postMessage({
          type: MESSAGE_TYPES.MASTER_HEARTBEAT,
          tabId: this.myTabId
        })
      }
    }, 5000) // 每5秒发送心跳
  }

  /**
   * 启动心跳超时检测（从连接）
   */
  startHeartbeatTimeout() {
    if (this.heartbeatTimeout) {
      clearTimeout(this.heartbeatTimeout)
    }

    this.heartbeatTimeout = setTimeout(() => {
      console.warn('[WS Coordinator] Master heartbeat timeout, re-electing...')
      this.masterTabId = null
      this.startElection()
    }, 10000) // 10秒未收到心跳则重新选举
  }

  /**
   * 创建 WebSocket 连接
   */
  connectWebSocket() {
    if (!this.isMaster) {
      console.warn('[WS Coordinator] Only master should connect WebSocket')
      return
    }

    this.wsClient = createWebSocketClient()
    this.wsClient.onMessage((data) => this.handleWebSocketData(data))

    const success = this.wsClient.connect()
    if (!success) {
      console.error('[WS Coordinator] WebSocket connection failed, falling back to polling')
      this.startPolling()
    }
  }

  /**
   * 处理 WebSocket 数据
   */
  handleWebSocketData(data) {
    // 主连接：广播数据到所有从标签页
    if (this.channel && this.isMaster) {
      this.channel.postMessage({
        type: MESSAGE_TYPES.DATA_BROADCAST,
        payload: data
      })
    }

    // 本地也处理数据
    if (this.dataHandler) {
      this.dataHandler(data)
    }
  }

  /**
   * 处理 BroadcastChannel 消息
   */
  handleChannelMessage(message) {
    switch (message.type) {
      case MESSAGE_TYPES.ELECT_MASTER:
        // 如果是主连接，回复 IAM_MASTER
        if (this.isMaster) {
          this.channel.postMessage({
            type: MESSAGE_TYPES.IAM_MASTER,
            tabId: this.myTabId
          })
        }
        break

      case MESSAGE_TYPES.IAM_MASTER:
        // 有其他标签页声称是主连接
        if (!this.masterTabId && !this.isMaster) {
          // 成为从连接
          this.becomeSlave(message.tabId)
        } else if (this.isMaster && message.tabId !== this.myTabId) {
          // 多个主连接，根据ID大小决定（较小的胜出）
          if (message.tabId < this.myTabId) {
            console.log('[WS Coordinator] Demoting to slave')
            this.demoteToSlave(message.tabId)
          }
        }
        break

      case MESSAGE_TYPES.MASTER_HEARTBEAT:
        // 收到主连接心跳
        if (!this.isMaster && message.tabId === this.masterTabId) {
          // 重置心跳超时
          this.startHeartbeatTimeout()
        }
        break

      case MESSAGE_TYPES.DATA_BROADCAST:
        // 从连接：接收主连接广播的数据
        if (!this.isMaster && this.dataHandler) {
          this.dataHandler(message.payload)
        }
        break
    }
  }

  /**
   * 降级为从连接
   */
  demoteToSlave(newMasterTabId) {
    this.isMaster = false
    this.masterTabId = newMasterTabId

    // 关闭 WebSocket 连接
    if (this.wsClient) {
      this.wsClient.disconnect()
      this.wsClient = null
    }

    // 停止心跳
    if (this.heartbeatTimer) {
      clearInterval(this.heartbeatTimer)
      this.heartbeatTimer = null
    }

    // 开始作为从连接运行
    this.startHeartbeatTimeout()
  }

  /**
   * 启动轮询（降级策略）
   */
  startPolling() {
    // 轮询逻辑在 store 中实现
    console.log('[WS Coordinator] Starting polling mode')
  }

  /**
   * 销毁协调器
   */
  destroy() {
    // 关闭 WebSocket 连接
    if (this.wsClient) {
      this.wsClient.disconnect()
      this.wsClient = null
    }

    // 停止定时器
    if (this.heartbeatTimer) {
      clearInterval(this.heartbeatTimer)
      this.heartbeatTimer = null
    }

    if (this.electionTimer) {
      clearTimeout(this.electionTimer)
      this.electionTimer = null
    }

    if (this.heartbeatTimeout) {
      clearTimeout(this.heartbeatTimeout)
      this.heartbeatTimeout = null
    }

    // 关闭 BroadcastChannel
    if (this.channel) {
      this.channel.close()
      this.channel = null
    }

    this.isActive = false
  }

  /**
   * 获取连接状态信息
   */
  getStatus() {
    return {
      isMaster: this.isMaster,
      myTabId: this.myTabId,
      masterTabId: this.masterTabId,
      isActive: this.isActive,
      hasWebSocket: this.wsClient !== null,
    }
  }
}

/**
 * 创建 WebSocket 协调器实例
 * @returns {WebSocketCoordinator} WebSocket 协调器
 */
export function createWebSocketCoordinator() {
  return new WebSocketCoordinator()
}

export default WebSocketCoordinator
