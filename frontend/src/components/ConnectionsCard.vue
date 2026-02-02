<template>
  <StatCard
    title="活跃连接"
    :value="stats.active_connections"
    card-class="connections"
    :show-progress="true"
    :progress="getConnectionsProgress()"
  >
    <template #icon>◈</template>
    <template #subtitle>最大: {{ stats.max_connections }}</template>
  </StatCard>
</template>

<script setup>
import StatCard from './StatCard.vue'
import { useWebSocket } from '../composables/useWebSocket'

const { stats, getConnectionsProgress } = useWebSocket()
</script>

<style scoped>
/* 样式已由 StatCard 统一处理 */
</style>
