<template>
  <div class="user-stats-card">
    <div class="card-header">
      <h3 class="card-title">
        <span class="title-icon">👥</span>
        用户流量统计
      </h3>
      <span class="user-count">{{ userStats.length }} 个用户</span>
    </div>

    <div v-if="userStats.length === 0" class="empty-state">
      <p>暂无用户数据</p>
    </div>

    <div v-else class="table-container">
      <table class="user-table">
        <thead>
          <tr>
            <th @click="sortBy('uuid')">
              UUID
              <span class="sort-icon" :class="{ active: sortKey === 'uuid' }">
                {{ sortKey === 'uuid' ? (sortAsc ? '↑' : '↓') : '↕' }}
              </span>
            </th>
            <th @click="sortBy('email')">
              邮箱
              <span class="sort-icon" :class="{ active: sortKey === 'email' }">
                {{ sortKey === 'email' ? (sortAsc ? '↑' : '↓') : '↕' }}
              </span>
            </th>
            <th @click="sortBy('total_traffic')">
              总流量
              <span class="sort-icon" :class="{ active: sortKey === 'total_traffic' }">
                {{ sortKey === 'total_traffic' ? (sortAsc ? '↑' : '↓') : '↕' }}
              </span>
            </th>
            <th @click="sortBy('active_connections')">
              连接数
              <span class="sort-icon" :class="{ active: sortKey === 'active_connections' }">
                {{ sortKey === 'active_connections' ? (sortAsc ? '↑' : '↓') : '↕' }}
              </span>
            </th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="user in sortedUsers" :key="user.uuid" class="user-row">
            <td class="uuid-cell">
              <code>{{ formatUuid(user.uuid) }}</code>
            </td>
            <td class="email-cell">
              {{ user.email || '-' }}
            </td>
            <td class="traffic-cell">
              <span class="traffic-value">{{ user.total_traffic }}</span>
            </td>
            <td class="connections-cell">
              <span class="connections-badge" :class="{ active: user.active_connections > 0 }">
                {{ user.active_connections }}
              </span>
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  </div>
</template>

<script setup>
import { ref, computed } from 'vue'
import { useWebSocket } from '../composables/useWebSocket'

const { userStats } = useWebSocket()

const sortKey = ref('total_traffic')
const sortAsc = ref(false)

const parseTraffic = (trafficStr) => {
  const match = trafficStr.match(/^([\d.]+)\s*(B|KB|MB|GB|TB)$/)
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

const sortedUsers = computed(() => {
  const users = [...userStats.value]
  users.sort((a, b) => {
    let aVal, bVal

    switch (sortKey.value) {
      case 'uuid':
        aVal = a.uuid
        bVal = b.uuid
        break
      case 'email':
        aVal = a.email || ''
        bVal = b.email || ''
        break
      case 'total_traffic':
        aVal = parseTraffic(a.total_traffic)
        bVal = parseTraffic(b.total_traffic)
        break
      case 'active_connections':
        aVal = a.active_connections
        bVal = b.active_connections
        break
      default:
        return 0
    }

    if (typeof aVal === 'string') {
      return sortAsc.value ? aVal.localeCompare(bVal) : bVal.localeCompare(aVal)
    }
    return sortAsc.value ? aVal - bVal : bVal - aVal
  })
  return users
})

const sortBy = (key) => {
  if (sortKey.value === key) {
    sortAsc.value = !sortAsc.value
  } else {
    sortKey.value = key
    sortAsc.value = false
  }
}

const formatUuid = (uuid) => {
  if (uuid.length <= 8) return uuid
  return uuid.substring(0, 8) + '...'
}
</script>

<style scoped>
.user-stats-card {
  background: var(--bg-glass);
  border: 1px solid var(--border-color);
  border-radius: 12px;
  padding: 1.5rem;
  backdrop-filter: blur(20px);
  box-shadow: 0 4px 20px rgba(0, 0, 0, 0.3);
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 1rem;
}

.card-title {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  margin: 0;
  font-size: var(--font-size-lg);
  font-weight: 600;
  color: var(--text-primary);
}

.title-icon {
  font-size: 1.2em;
}

.user-count {
  font-size: var(--font-size-sm);
  color: var(--text-secondary);
  padding: 0.25rem 0.75rem;
  background: rgba(0, 245, 255, 0.1);
  border: 1px solid var(--neon-cyan);
  border-radius: 9999px;
  color: var(--neon-cyan);
}

.empty-state {
  text-align: center;
  padding: 2rem;
  color: var(--text-secondary);
}

.table-container {
  overflow-x: auto;
}

.user-table {
  width: 100%;
  border-collapse: collapse;
  font-size: var(--font-size-sm);
}

.user-table thead {
  background: rgba(0, 245, 255, 0.05);
  border-bottom: 1px solid var(--neon-cyan);
}

.user-table th {
  padding: 0.75rem;
  text-align: left;
  font-weight: 600;
  color: var(--neon-cyan);
  cursor: pointer;
  user-select: none;
  transition: background 0.2s ease;
}

.user-table th:hover {
  background: rgba(0, 245, 255, 0.1);
}

.sort-icon {
  margin-left: 0.5rem;
  opacity: 0.3;
  transition: opacity 0.2s ease;
}

.sort-icon.active {
  opacity: 1;
}

.user-row {
  border-bottom: 1px solid var(--border-color);
  transition: background 0.2s ease;
}

.user-row:hover {
  background: rgba(0, 245, 255, 0.03);
}

.user-table td {
  padding: 0.75rem;
  color: var(--text-primary);
}

.uuid-cell code {
  background: rgba(0, 0, 0, 0.3);
  padding: 0.25rem 0.5rem;
  border-radius: 4px;
  font-family: 'Courier New', monospace;
  font-size: var(--font-size-xs);
  color: var(--neon-cyan);
}

.traffic-cell {
  font-weight: 600;
}

.traffic-value {
  color: var(--neon-pink);
}

.connections-badge {
  display: inline-block;
  padding: 0.25rem 0.75rem;
  background: rgba(255, 255, 255, 0.05);
  border: 1px solid var(--border-color);
  border-radius: 9999px;
  font-weight: 600;
  min-width: 40px;
  text-align: center;
  transition: all 0.2s ease;
}

.connections-badge.active {
  background: rgba(0, 245, 255, 0.2);
  border-color: var(--neon-cyan);
  color: var(--neon-cyan);
  box-shadow: 0 0 10px rgba(0, 245, 255, 0.3);
}

@media (max-width: 768px) {
  .user-stats-card {
    padding: 1rem;
  }

  .card-header {
    flex-direction: column;
    align-items: flex-start;
    gap: 0.5rem;
  }

  .user-table {
    font-size: var(--font-size-xs);
  }

  .user-table th,
  .user-table td {
    padding: 0.5rem;
  }
}
</style>
