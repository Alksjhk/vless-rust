<template>
  <StatCard
    title="内存使用"
    :value="stats.memory_usage"
    card-class="memory"
    :show-progress="true"
    :progress="getMemoryProgress()"
  >
    <template #icon>◇</template>
    <template #subtitle>总内存: {{ stats.total_memory }}</template>
  </StatCard>
</template>

<script setup>
import StatCard from './StatCard.vue'
import { useWebSocket } from '../composables/useWebSocket'

const { stats, getMemoryProgress } = useWebSocket()
</script>

<style scoped>
/* 样式已由 StatCard 统一处理 */
</style>
