<template>
  <StatCard
    title="下载速度"
    :value="stats.download_speed"
    card-class="download"
    :show-progress="true"
    :progress="getDownloadProgress()"
  >
    <template #icon>▼</template>
    <template #subtitle>峰值: {{ formatBytes(downloadPeak) }}/s</template>
  </StatCard>
</template>

<script setup>
import StatCard from './StatCard.vue'
import { useWebSocket } from '../composables/useWebSocket'

const { stats, downloadPeak, formatBytes, getDownloadProgress } = useWebSocket()
</script>

<style scoped>
/* 样式已由 StatCard 统一处理 */
</style>
