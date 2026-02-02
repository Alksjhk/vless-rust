<template>
  <StatCard
    title="运行时间"
    :value="stats.uptime"
    card-class="uptime"
    :subtitle="subtitle"
  >
    <template #icon>⏱</template>
  </StatCard>
</template>

<script setup>
import { computed } from 'vue'
import StatCard from './StatCard.vue'
import { useWebSocket } from '../composables/useWebSocket'

const { stats } = useWebSocket()

const subtitle = computed(() => {
  if (stats.start_time) {
    return `启动于: ${new Date(stats.start_time).toLocaleDateString()}`
  }
  return '启动中...'
})
</script>

<style scoped>
/* 样式已由 StatCard 统一处理 */
</style>
