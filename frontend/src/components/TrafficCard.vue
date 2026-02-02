<template>
  <StatCard
    title="总流量"
    :value="stats.total_traffic"
    card-class="traffic"
    subtitle="实时更新"
  >
    <template #icon>◈</template>
  </StatCard>
</template>

<script setup>
import StatCard from './StatCard.vue'
import { useWebSocket } from '../composables/useWebSocket'

const { stats } = useWebSocket()
</script>

<style scoped>
/* 样式已由 StatCard 统一处理 */
</style>
