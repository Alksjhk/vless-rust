<template>
  <div class="waveform-chart">
    <div class="chart-header">
      <div class="chart-title">
        <span class="title-icon">⚡</span>
        <span>实时波形</span>
      </div>
      <div class="speed-indicators">
        <div class="speed-item upload">
          <div class="speed-label">上传</div>
          <div class="speed-value">{{ currentUploadSpeed }}</div>
        </div>
        <div class="speed-item download">
          <div class="speed-label">下载</div>
          <div class="speed-value">{{ currentDownloadSpeed }}</div>
        </div>
      </div>
    </div>

    <div class="canvas-container" ref="containerRef">
      <canvas
        ref="canvasRef"
        @mousemove="handleMouseMove"
        @mouseleave="hideTooltip"
      ></canvas>

      <div
        v-if="showTooltip"
        class="tooltip"
        :style="tooltipStyle"
      >
        <div class="tooltip-time">{{ formatTime(hoverTime) }}</div>
        <div class="tooltip-row upload">
          <span class="label">上传:</span>
          <span class="value">{{ props.formatBytes(hoverData.upload) }}/s</span>
        </div>
        <div class="tooltip-row download">
          <span class="label">下载:</span>
          <span class="value">{{ props.formatBytes(hoverData.download) }}/s</span>
        </div>
      </div>

      <div class="grid-overlay"></div>
    </div>

    <div class="chart-footer">
      <div class="chart-legend">
        <div class="legend-item">
          <div class="legend-dot upload"></div>
          <span>上传通道</span>
        </div>
        <div class="legend-item">
          <div class="legend-dot download"></div>
          <span>下载通道</span>
        </div>
      </div>
      <div class="peak-info">
        <span>峰值: {{ peakUpload }} / {{ peakDownload }}</span>
      </div>
    </div>
  </div>
</template>

<script setup>
import { computed, ref, onMounted, onUnmounted, watch, nextTick } from 'vue'

const props = defineProps({
  chartData: {
    type: Array,
    default: () => []
  },
  formatBytes: {
    type: Function,
    required: true
  },
  pollInterval: {
    type: Number,
    default: 1000
  }
})

const canvasRef = ref(null)
const containerRef = ref(null)
const showTooltip = ref(false)
const hoverIndex = ref(-1)
const animationId = ref(null)
const timeOffset = ref(0)
const mouseX = ref(0)
const mouseY = ref(0)

const currentUploadSpeed = computed(() => {
  const data = props.chartData
  if (data.length === 0) return '--'
  return props.formatBytes(data[data.length - 1].upload) + '/s'
})

const currentDownloadSpeed = computed(() => {
  const data = props.chartData
  if (data.length === 0) return '--'
  return props.formatBytes(data[data.length - 1].download) + '/s'
})

const peakUpload = computed(() => {
  const data = props.chartData
  if (data.length === 0) return '--'
  const max = Math.max(...data.map(d => d.upload))
  return props.formatBytes(max) + '/s'
})

const peakDownload = computed(() => {
  const data = props.chartData
  if (data.length === 0) return '--'
  const max = Math.max(...data.map(d => d.download))
  return props.formatBytes(max) + '/s'
})

const hoverData = computed(() => {
  if (hoverIndex.value < 0 || hoverIndex.value >= props.chartData.length) {
    return { upload: 0, download: 0 }
  }
  return props.chartData[hoverIndex.value]
})

const hoverTime = computed(() => {
  if (hoverIndex.value < 0) return 0
  return (props.chartData.length - 1 - hoverIndex.value) * (props.pollInterval / 1000)
})

const tooltipStyle = computed(() => {
  if (!containerRef.value || !canvasRef.value) return {}

  const containerRect = containerRef.value.getBoundingClientRect()
  const canvasRect = canvasRef.value.getBoundingClientRect()

  // 计算 tooltip 尺寸（估算值）
  const tooltipWidth = 180
  const tooltipHeight = 100

  // 计算数据点的 X 位置
  const xPercent = hoverIndex.value / (props.chartData.length - 1)
  const pointX = xPercent * canvasRect.width

  // 计算 left 位置，默认在鼠标右侧
  let left = pointX + 15
  // 如果右侧空间不足，显示在左侧
  if (left + tooltipWidth > canvasRect.width) {
    left = pointX - tooltipWidth - 15
  }
  // 确保不超出左边界
  if (left < 0) left = 10

  // 计算 top 位置，跟随鼠标位置
  let top = mouseY.value + 15
  // 如果底部空间不足，显示在上方
  if (top + tooltipHeight > canvasRect.height) {
    top = mouseY.value - tooltipHeight - 15
  }
  // 确保在边界内
  if (top < 0) top = 10
  if (top + tooltipHeight > canvasRect.height) {
    top = canvasRect.height - tooltipHeight - 10
  }

  return {
    left: left + 'px',
    top: top + 'px'
  }
})

const formatTime = (seconds) => {
  if (seconds < 60) return `${Math.round(seconds)}秒前`
  const mins = Math.floor(seconds / 60)
  const secs = Math.round(seconds % 60)
  return `${mins}分${secs}秒前`
}

const handleMouseMove = (event) => {
  if (props.chartData.length < 2) return

  const rect = canvasRef.value.getBoundingClientRect()
  const x = event.clientX - rect.left
  const y = event.clientY - rect.top
  const xPercent = x / rect.width

  mouseX.value = y
  mouseY.value = y
  hoverIndex.value = Math.round(xPercent * (props.chartData.length - 1))
  showTooltip.value = true
}

const hideTooltip = () => {
  showTooltip.value = false
  hoverIndex.value = -1
}

const drawWaveform = (timestamp) => {
  const canvas = canvasRef.value
  if (!canvas) return

  const ctx = canvas.getContext('2d')
  const container = containerRef.value

  // 设置高 DPI
  const dpr = window.devicePixelRatio || 1
  const rect = container.getBoundingClientRect()

  canvas.width = rect.width * dpr
  canvas.height = rect.height * dpr
  canvas.style.width = rect.width + 'px'
  canvas.style.height = rect.height + 'px'
  ctx.scale(dpr, dpr)

  const width = rect.width
  const height = rect.height

  // 清空画布
  ctx.clearRect(0, 0, width, height)

  if (props.chartData.length < 2) {
    animationId.value = requestAnimationFrame(drawWaveform)
    return
  }

  // 计算最大值
  const maxSpeed = Math.max(
    ...props.chartData.map(d => Math.max(d.upload, d.download)),
    1
  )
  const padding = maxSpeed * 0.1
  const yMax = maxSpeed + padding

  // 绘制网格
  ctx.strokeStyle = 'rgba(0, 245, 255, 0.1)'
  ctx.lineWidth = 1
  const gridLines = 5
  for (let i = 0; i <= gridLines; i++) {
    const y = (height / gridLines) * i
    ctx.beginPath()
    ctx.moveTo(0, y)
    ctx.lineTo(width, y)
    ctx.stroke()
  }

  // 绘制上传波形 (霓虹粉)
  drawWave(ctx, props.chartData, 'upload', width, height, yMax, {
    color: '#FF2D95',
    glow: 'rgba(255, 45, 149, 0.6)',
    lineWidth: 2.5
  })

  // 绘制下载波形 (霓虹青)
  drawWave(ctx, props.chartData, 'download', width, height, yMax, {
    color: '#00F5FF',
    glow: 'rgba(0, 245, 255, 0.6)',
    lineWidth: 2.5
  })

  // 绘制悬停线
  if (hoverIndex.value >= 0) {
    const x = (hoverIndex.value / (props.chartData.length - 1)) * width
    ctx.strokeStyle = 'rgba(255, 255, 255, 0.5)'
    ctx.lineWidth = 1
    ctx.setLineDash([5, 5])
    ctx.beginPath()
    ctx.moveTo(x, 0)
    ctx.lineTo(x, height)
    ctx.stroke()
    ctx.setLineDash([])
  }

  animationId.value = requestAnimationFrame(drawWaveform)
}

const drawWave = (ctx, data, key, width, height, yMax, style) => {
  const points = data.map((d, i) => ({
    x: (i / (data.length - 1)) * width,
    y: height - (d[key] / yMax) * height * 0.9 - height * 0.05
  }))

  // 绘制发光效果
  ctx.shadowColor = style.glow
  ctx.shadowBlur = 15

  // 绘制波形线
  ctx.strokeStyle = style.color
  ctx.lineWidth = style.lineWidth
  ctx.lineCap = 'round'
  ctx.lineJoin = 'round'
  ctx.beginPath()
  ctx.moveTo(points[0].x, points[0].y)

  for (let i = 1; i < points.length; i++) {
    const xc = (points[i].x + points[i - 1].x) / 2
    const yc = (points[i].y + points[i - 1].y) / 2
    ctx.quadraticCurveTo(points[i - 1].x, points[i - 1].y, xc, yc)
  }
  ctx.lineTo(points[points.length - 1].x, points[points.length - 1].y)
  ctx.stroke()

  // 绘制填充渐变
  ctx.shadowBlur = 0
  const gradient = ctx.createLinearGradient(0, 0, 0, height)
  gradient.addColorStop(0, style.glow)
  gradient.addColorStop(1, 'transparent')

  ctx.fillStyle = gradient
  ctx.globalAlpha = 0.2
  ctx.beginPath()
  ctx.moveTo(points[0].x, height)
  ctx.lineTo(points[0].x, points[0].y)

  for (let i = 1; i < points.length; i++) {
    const xc = (points[i].x + points[i - 1].x) / 2
    const yc = (points[i].y + points[i - 1].y) / 2
    ctx.quadraticCurveTo(points[i - 1].x, points[i - 1].y, xc, yc)
  }
  ctx.lineTo(points[points.length - 1].x, points[points.length - 1].y)
  ctx.lineTo(points[points.length - 1].x, height)
  ctx.closePath()
  ctx.fill()
  ctx.globalAlpha = 1

  // 绘制数据点
  if (hoverIndex.value >= 0) {
    const point = points[hoverIndex.value]
    ctx.shadowColor = style.glow
    ctx.shadowBlur = 10
    ctx.fillStyle = style.color
    ctx.beginPath()
    ctx.arc(point.x, point.y, 4, 0, Math.PI * 2)
    ctx.fill()
    ctx.shadowBlur = 0
  }
}

watch(() => props.chartData, () => {
  // 数据更新时触发重绘
}, { deep: true })

onMounted(() => {
  nextTick(() => {
    animationId.value = requestAnimationFrame(drawWaveform)
  })
})

onUnmounted(() => {
  if (animationId.value) {
    cancelAnimationFrame(animationId.value)
  }
})
</script>

<style scoped>
.waveform-chart {
  background: var(--bg-glass);
  border: 1px solid var(--border-color);
  border-radius: 12px;
  padding: 1.25rem;
  backdrop-filter: blur(20px);
  box-shadow:
    0 0 30px rgba(0, 245, 255, 0.1),
    inset 0 1px 0 rgba(255, 255, 255, 0.05);
}

.chart-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 1rem;
  flex-wrap: wrap;
  gap: 1rem;
}

.chart-title {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  font-size: var(--font-size-lg);
  font-weight: 600;
  color: var(--text-primary);
  text-transform: uppercase;
  letter-spacing: 1px;
}

.title-icon {
  font-size: 1.2em;
  animation: pulse 2s ease-in-out infinite;
}

@keyframes pulse {
  0%, 100% { opacity: 1; text-shadow: var(--glow-cyan); }
  50% { opacity: 0.7; text-shadow: none; }
}

.speed-indicators {
  display: flex;
  gap: 2rem;
}

.speed-item {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 0.25rem;
}

.speed-label {
  font-size: var(--font-size-xs);
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 1px;
}

.speed-value {
  font-size: var(--font-size-lg);
  font-weight: 700;
  font-family: var(--font-family);
  color: var(--text-primary);
}

.speed-item.upload .speed-value {
  color: var(--neon-pink);
  text-shadow: var(--glow-pink);
}

.speed-item.download .speed-value {
  color: var(--neon-cyan);
  text-shadow: var(--glow-cyan);
}

.canvas-container {
  position: relative;
  height: 220px;
  border-radius: 8px;
  overflow: hidden;
  background: rgba(0, 0, 0, 0.2);
}

canvas {
  width: 100%;
  height: 100%;
  cursor: crosshair;
}

.grid-overlay {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  pointer-events: none;
  background-image:
    linear-gradient(rgba(0, 245, 255, 0.03) 1px, transparent 1px),
    linear-gradient(90deg, rgba(0, 245, 255, 0.03) 1px, transparent 1px);
  background-size: 50px 50px;
}

.tooltip {
  position: absolute;
  background: rgba(5, 5, 16, 0.95);
  border: 1px solid var(--border-color);
  border-radius: 8px;
  padding: 0.75rem;
  font-size: var(--font-size-xs);
  pointer-events: none;
  z-index: 100;
  box-shadow: 0 0 20px rgba(0, 245, 255, 0.3);
  backdrop-filter: blur(10px);
  transition: opacity 0.2s ease;
}

.tooltip-time {
  font-weight: 600;
  margin-bottom: 0.5rem;
  color: var(--text-primary);
  font-size: var(--font-size-sm);
}

.tooltip-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  line-height: 1.6;
}

.tooltip-row.upload .value {
  color: var(--neon-pink);
  text-shadow: var(--glow-pink);
}

.tooltip-row.download .value {
  color: var(--neon-cyan);
  text-shadow: var(--glow-cyan);
}

.tooltip-row .label {
  color: var(--text-secondary);
}

.tooltip-row .value {
  font-weight: 600;
  font-family: var(--font-family);
}

.chart-footer {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-top: 1rem;
  padding-top: 0.75rem;
  border-top: 1px solid var(--border-color);
  flex-wrap: wrap;
  gap: 0.75rem;
}

.chart-legend {
  display: flex;
  gap: 1.5rem;
}

.legend-item {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  font-size: var(--font-size-sm);
  color: var(--text-secondary);
}

.legend-dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  position: relative;
}

.legend-dot.upload {
  background: var(--neon-pink);
  box-shadow: var(--glow-pink);
}

.legend-dot.download {
  background: var(--neon-cyan);
  box-shadow: var(--glow-cyan);
}

.peak-info {
  font-size: var(--font-size-xs);
  color: var(--text-secondary);
  font-family: var(--font-family);
}

@media (max-width: 640px) {
  .waveform-chart {
    padding: 1rem;
  }

  .chart-header {
    flex-direction: column;
    align-items: flex-start;
  }

  .speed-indicators {
    width: 100%;
    justify-content: space-between;
    gap: 1rem;
  }

  .speed-item {
    align-items: flex-start;
  }

  .canvas-container {
    height: 160px;
  }

  .chart-footer {
    flex-direction: column;
    align-items: flex-start;
    gap: 0.5rem;
  }
}

@media (prefers-reduced-motion: reduce) {
  .title-icon {
    animation: none;
  }
}
</style>
