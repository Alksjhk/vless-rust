<template>
  <StatCard
    title="上传速度"
    :value="stats.upload_speed"
    card-class="upload"
    :show-progress="true"
    :progress="getUploadProgress()"
  >
    <template #icon>▲</template>
    <template #subtitle>峰值: {{ formatBytes(uploadPeak) }}/s</template>
  </StatCard>
</template>

<script setup>
import StatCard from './StatCard.vue'
import { useWebSocket } from '../composables/useWebSocket'

const { stats, uploadPeak, formatBytes, getUploadProgress } = useWebSocket()
</script>

<style scoped>
/* 样式已由 StatCard 统一处理 */
</style>
