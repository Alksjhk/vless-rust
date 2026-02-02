<template>
  <div class="app">
    <header class="header">
      <div class="header-left">
        <h1 class="title">
          <span class="title-icon">⚡</span>
          VLESS 监控
        </h1>
        <div class="connection-status" :class="{ connected: connected, disconnected: error }">
          <span class="status-dot"></span>
          <span class="status-text">{{ statusText }}</span>
        </div>
      </div>
      <ThemeToggle />
    </header>

    <main class="container">
      <div v-if="loading" class="loading-overlay">
        <div class="spinner"></div>
        <p>连接中...</p>
      </div>

      <div v-if="error && !loading" class="error-banner">
        <p>{{ error }}</p>
        <button @click="reconnect" class="reconnect-btn">重新连接</button>
      </div>

      <!-- 波形图放在顶部 -->
      <TrafficChart :chart-data="getChartData()" :format-bytes="formatBytes" :poll-interval="1000" />

      <!-- 卡片网格 3 列布局 -->
      <div class="grid">
        <SpeedCard />
        <DownloadCard />
        <TrafficCard />
        <UptimeCard />
        <MemoryCard />
        <ConnectionsCard />
      </div>
    </main>
  </div>
</template>

<script setup>
import { computed } from 'vue'
import ThemeToggle from './components/ThemeToggle.vue'
import SpeedCard from './components/SpeedCard.vue'
import DownloadCard from './components/DownloadCard.vue'
import TrafficCard from './components/TrafficCard.vue'
import UptimeCard from './components/UptimeCard.vue'
import MemoryCard from './components/MemoryCard.vue'
import ConnectionsCard from './components/ConnectionsCard.vue'
import TrafficChart from './components/TrafficChart.vue'
import { useWebSocket } from './composables/useWebSocket'

const { loading, error, connected, reconnect, formatBytes, getChartData } = useWebSocket()

const statusText = computed(() => {
  if (loading.value) return '连接中...'
  if (error.value) return '已断开'
  if (connected.value) return '已连接'
  return '未连接'
})
</script>

<style scoped>
.app {
  min-height: 100vh;
  background: var(--bg-primary);
}

.header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 1.5rem 2rem;
  background: var(--bg-glass);
  border-bottom: 1px solid var(--border-color);
  backdrop-filter: blur(20px);
  position: sticky;
  top: 0;
  z-index: 100;
}

.header-left {
  display: flex;
  align-items: center;
  gap: 2rem;
}

.title {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  font-size: var(--font-size-xl);
  font-weight: 700;
  margin: 0;
  color: var(--text-primary);
  text-transform: uppercase;
  letter-spacing: 2px;
}

.title-icon {
  font-size: 1.5em;
  animation: pulse 2s ease-in-out infinite;
}

@keyframes pulse {
  0%, 100% {
    opacity: 1;
    text-shadow: var(--glow-cyan);
  }
  50% {
    opacity: 0.7;
    text-shadow: none;
  }
}

.connection-status {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  font-size: var(--font-size-sm);
  padding: 0.35rem 0.85rem;
  border-radius: 9999px;
  background: rgba(255, 255, 255, 0.05);
  border: 1px solid var(--border-color);
  transition: all 0.3s ease;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 1px;
}

.connection-status.connected {
  background: rgba(0, 245, 255, 0.1);
  border-color: var(--neon-cyan);
  color: var(--neon-cyan);
  box-shadow: 0 0 20px rgba(0, 245, 255, 0.3);
}

.connection-status.disconnected {
  background: rgba(255, 45, 149, 0.1);
  border-color: var(--neon-pink);
  color: var(--neon-pink);
  box-shadow: 0 0 20px rgba(255, 45, 149, 0.3);
}

.status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: currentColor;
  animation: dotPulse 2s ease-in-out infinite;
}

.connection-status.disconnected .status-dot {
  animation: none;
}

@keyframes dotPulse {
  0%, 100% {
    opacity: 1;
    transform: scale(1);
  }
  50% {
    opacity: 0.5;
    transform: scale(0.8);
  }
}

.error-banner {
  position: fixed;
  top: 1rem;
  left: 50%;
  transform: translateX(-50%);
  background: rgba(255, 45, 149, 0.9);
  border: 1px solid var(--neon-pink);
  color: white;
  padding: 1rem 1.5rem;
  border-radius: 8px;
  display: flex;
  align-items: center;
  gap: 1rem;
  z-index: 1000;
  animation: slideDown 0.3s ease;
  box-shadow: 0 0 30px rgba(255, 45, 149, 0.5);
  backdrop-filter: blur(10px);
}

@keyframes slideDown {
  from {
    transform: translateX(-50%) translateY(-100%);
    opacity: 0;
  }
  to {
    transform: translateX(-50%) translateY(0);
    opacity: 1;
  }
}

.error-banner p {
  margin: 0;
  font-weight: 600;
}

.reconnect-btn {
  padding: 0.5rem 1rem;
  background: rgba(0, 0, 0, 0.3);
  color: white;
  border: 1px solid var(--neon-pink);
  border-radius: 4px;
  cursor: pointer;
  font-weight: 600;
  transition: all 0.2s ease;
  text-transform: uppercase;
  letter-spacing: 1px;
  font-size: var(--font-size-xs);
}

.reconnect-btn:hover {
  background: var(--neon-pink);
  box-shadow: var(--glow-pink);
}

.container {
  max-width: 1400px;
  margin: 0 auto;
  padding: 2rem;
}

.grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 1.5rem;
  margin-top: 1.5rem;
}

.loading-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: var(--bg-primary);
  display: flex;
  flex-direction: column;
  justify-content: center;
  align-items: center;
  gap: 1rem;
  z-index: 1000;
}

.loading-overlay p {
  margin: 0;
  color: var(--text-secondary);
  font-size: var(--font-size-sm);
  text-transform: uppercase;
  letter-spacing: 2px;
}

.spinner {
  width: 50px;
  height: 50px;
  border: 3px solid rgba(0, 245, 255, 0.1);
  border-top-color: var(--neon-cyan);
  border-radius: 50%;
  animation: spin 1s linear infinite;
  box-shadow: 0 0 20px rgba(0, 245, 255, 0.3);
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

@media (max-width: 1024px) {
  .grid {
    grid-template-columns: repeat(2, 1fr);
  }
}

@media (max-width: 640px) {
  .grid {
    grid-template-columns: 1fr;
  }

  .container {
    padding: 1rem;
  }

  .header {
    padding: 1rem;
  }

  .header-left {
    flex-direction: column;
    align-items: flex-start;
    gap: 0.75rem;
  }

  .title {
    font-size: var(--font-size-lg);
  }

  .title-icon {
    font-size: 1.3em;
  }

  .connection-status {
    font-size: var(--font-size-xs);
    padding: 0.25rem 0.6rem;
  }

  .error-banner {
    left: 1rem;
    right: 1rem;
    top: 1rem;
    transform: none;
    flex-direction: column;
    text-align: center;
  }

  @keyframes slideDown {
    from {
      transform: translateY(-100%);
      opacity: 0;
    }
    to {
      transform: translateY(0);
      opacity: 1;
    }
  }
}

@media (prefers-reduced-motion: reduce) {
  .spinner {
    animation: none;
  }

  .status-dot {
    animation: none;
  }

  .title-icon {
    animation: none;
  }

  .reconnect-btn {
    transition: none;
  }

  .error-banner {
    animation: none;
  }
}
</style>
