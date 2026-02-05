<template>
  <div class="stat-card">
    <div class="card-header">
      <div class="card-icon">🔗</div>
      <div class="card-title">连接池</div>
    </div>
    <div class="card-content">
      <div v-if="poolData" class="pool-stats">
        <div class="stat-row">
          <span class="stat-label">缓存命中率</span>
          <span class="stat-value hit-rate" :class="getHitRateClass(poolData.hit_rate)">
            {{ poolData.hit_rate.toFixed(1) }}%
          </span>
        </div>
        <div class="stat-row">
          <span class="stat-label">活跃连接</span>
          <span class="stat-value">{{ poolData.current_active }}</span>
        </div>
        <div class="stat-row">
          <span class="stat-label">空闲连接</span>
          <span class="stat-value">{{ poolData.current_idle }}</span>
        </div>
        <div class="stat-row">
          <span class="stat-label">已创建</span>
          <span class="stat-value">{{ poolData.total_created }}</span>
        </div>
        <div class="stat-row">
          <span class="stat-label">已复用</span>
          <span class="stat-value">{{ poolData.total_reused }}</span>
        </div>
        <div class="stat-row">
          <span class="stat-label">已关闭</span>
          <span class="stat-value">{{ poolData.total_closed }}</span>
        </div>
        <div class="stat-row">
          <span class="stat-label">缓存命中</span>
          <span class="stat-value">{{ poolData.cache_hits }}</span>
        </div>
        <div class="stat-row">
          <span class="stat-label">缓存未命中</span>
          <span class="stat-value">{{ poolData.cache_misses }}</span>
        </div>
      </div>
      <div v-else class="no-data">
        <span>连接池数据不可用</span>
      </div>
    </div>
  </div>
</template>

<script setup>
import { computed } from 'vue'
import { useWebSocket } from '../composables/useWebSocket'

const { data } = useWebSocket()

const poolData = computed(() => {
  return data.value?.connection_pool || null
})

const getHitRateClass = (hitRate) => {
  if (hitRate >= 80) return 'excellent'
  if (hitRate >= 60) return 'good'
  if (hitRate >= 40) return 'fair'
  return 'poor'
}
</script>

<style scoped>
.stat-card {
  background: var(--bg-glass);
  border: 1px solid var(--border-color);
  border-radius: 12px;
  padding: 1.5rem;
  backdrop-filter: blur(20px);
  transition: all 0.3s ease;
  position: relative;
  overflow: hidden;
}

.stat-card::before {
  content: '';
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  height: 2px;
  background: linear-gradient(90deg, var(--neon-cyan), var(--neon-purple));
  opacity: 0.6;
}

.stat-card:hover {
  transform: translateY(-2px);
  box-shadow: 0 8px 32px rgba(0, 245, 255, 0.15);
  border-color: var(--neon-cyan);
}

.card-header {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  margin-bottom: 1rem;
}

.card-icon {
  font-size: 1.5rem;
  filter: drop-shadow(0 0 8px currentColor);
}

.card-title {
  font-size: var(--font-size-lg);
  font-weight: 700;
  color: var(--text-primary);
  text-transform: uppercase;
  letter-spacing: 1px;
}

.card-content {
  color: var(--text-secondary);
}

.pool-stats {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.stat-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0.5rem 0;
  border-bottom: 1px solid rgba(255, 255, 255, 0.05);
}

.stat-row:last-child {
  border-bottom: none;
}

.stat-label {
  font-size: var(--font-size-sm);
  color: var(--text-secondary);
  font-weight: 500;
}

.stat-value {
  font-size: var(--font-size-sm);
  font-weight: 700;
  color: var(--text-primary);
  font-family: 'Courier New', monospace;
}

.hit-rate {
  padding: 0.25rem 0.5rem;
  border-radius: 4px;
  font-size: var(--font-size-xs);
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 1px;
}

.hit-rate.excellent {
  background: rgba(0, 255, 127, 0.2);
  color: #00ff7f;
  box-shadow: 0 0 10px rgba(0, 255, 127, 0.3);
}

.hit-rate.good {
  background: rgba(0, 245, 255, 0.2);
  color: var(--neon-cyan);
  box-shadow: 0 0 10px rgba(0, 245, 255, 0.3);
}

.hit-rate.fair {
  background: rgba(255, 215, 0, 0.2);
  color: #ffd700;
  box-shadow: 0 0 10px rgba(255, 215, 0, 0.3);
}

.hit-rate.poor {
  background: rgba(255, 45, 149, 0.2);
  color: var(--neon-pink);
  box-shadow: 0 0 10px rgba(255, 45, 149, 0.3);
}

.no-data {
  text-align: center;
  padding: 2rem 0;
  color: var(--text-secondary);
  font-style: italic;
}

@media (max-width: 640px) {
  .stat-card {
    padding: 1rem;
  }

  .card-header {
    margin-bottom: 0.75rem;
  }

  .card-icon {
    font-size: 1.25rem;
  }

  .card-title {
    font-size: var(--font-size-base);
  }

  .pool-stats {
    gap: 0.5rem;
  }

  .stat-row {
    padding: 0.375rem 0;
  }

  .stat-label,
  .stat-value {
    font-size: var(--font-size-xs);
  }

  .hit-rate {
    padding: 0.2rem 0.4rem;
    font-size: 10px;
  }
}
</style>