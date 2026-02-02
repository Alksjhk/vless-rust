<template>
  <div :class="['cyber-card', cardClass]">
    <div class="card-header">
      <slot name="icon">
        <span class="default-icon">◆</span>
      </slot>
      <span class="card-title">{{ title }}</span>
    </div>
    <div class="card-value">{{ value }}</div>
    <div v-if="subtitle" class="card-subtitle">
      <slot name="subtitle">{{ subtitle }}</slot>
    </div>
    <div v-if="showProgress" class="progress-bar">
      <div class="progress-fill" :style="{ width: progress + '%' }"></div>
      <div class="progress-glow" :style="{ width: progress + '%' }"></div>
    </div>
    <div class="card-decoration">
      <div class="decoration-corner top-left"></div>
      <div class="decoration-corner top-right"></div>
      <div class="decoration-corner bottom-left"></div>
      <div class="decoration-corner bottom-right"></div>
    </div>
  </div>
</template>

<script setup>
defineProps({
  title: {
    type: String,
    required: true
  },
  value: {
    type: [String, Number],
    required: true
  },
  subtitle: {
    type: String,
    default: ''
  },
  cardClass: {
    type: String,
    default: ''
  },
  showProgress: {
    type: Boolean,
    default: false
  },
  progress: {
    type: Number,
    default: 0
  }
})
</script>

<style scoped>
.cyber-card {
  position: relative;
  background: var(--bg-glass);
  border: 1px solid var(--border-color);
  border-radius: 12px;
  padding: 1.25rem;
  backdrop-filter: blur(20px);
  transition: all 0.3s ease;
  overflow: hidden;
}

.cyber-card::before {
  content: '';
  position: absolute;
  top: 0;
  left: -100%;
  width: 100%;
  height: 100%;
  background: linear-gradient(
    90deg,
    transparent,
    rgba(0, 245, 255, 0.1),
    transparent
  );
  transition: left 0.5s ease;
}

.cyber-card:hover {
  transform: translateY(-2px);
  border-color: var(--border-glow);
  box-shadow:
    0 0 30px rgba(0, 245, 255, 0.15),
    inset 0 1px 0 rgba(255, 255, 255, 0.1);
}

.cyber-card:hover::before {
  left: 100%;
}

.card-header {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  margin-bottom: 0.75rem;
  color: var(--text-secondary);
  font-size: var(--font-size-xs);
  text-transform: uppercase;
  letter-spacing: 1px;
}

.card-header :deep(.default-icon) {
  font-size: 1.2em;
  color: var(--neon-cyan);
  text-shadow: var(--glow-cyan);
}

.card-title {
  font-weight: 600;
}

.card-value {
  font-size: var(--font-size-2xl);
  font-weight: 700;
  font-family: var(--font-family);
  margin-bottom: 0.5rem;
  color: var(--text-primary);
  text-shadow: 0 0 20px rgba(0, 245, 255, 0.3);
}

.card-subtitle {
  color: var(--text-secondary);
  font-size: var(--font-size-xs);
  opacity: 0.8;
}

.progress-bar {
  position: relative;
  height: 6px;
  background: rgba(0, 0, 0, 0.3);
  border-radius: 3px;
  overflow: hidden;
  margin-top: 1rem;
}

.progress-fill {
  position: relative;
  height: 100%;
  background: linear-gradient(90deg, var(--neon-cyan), var(--neon-blue));
  border-radius: 3px;
  transition: width 0.5s ease;
  z-index: 2;
}

.progress-glow {
  position: absolute;
  top: 0;
  left: 0;
  height: 100%;
  background: inherit;
  filter: blur(8px);
  opacity: 0.8;
  z-index: 1;
}

.card-decoration {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  pointer-events: none;
  overflow: hidden;
}

.decoration-corner {
  position: absolute;
  width: 8px;
  height: 8px;
  border: 1px solid var(--neon-cyan);
  opacity: 0.5;
}

.decoration-corner.top-left {
  top: 4px;
  left: 4px;
  border-right: none;
  border-bottom: none;
}

.decoration-corner.top-right {
  top: 4px;
  right: 4px;
  border-left: none;
  border-bottom: none;
}

.decoration-corner.bottom-left {
  bottom: 4px;
  left: 4px;
  border-right: none;
  border-top: none;
}

.decoration-corner.bottom-right {
  bottom: 4px;
  right: 4px;
  border-left: none;
  border-top: none;
}

/* 卡片类型样式 */
.cyber-card.upload :deep(.card-value) {
  color: var(--neon-pink);
  text-shadow: var(--glow-pink);
}

.cyber-card.upload :deep(.progress-fill) {
  background: linear-gradient(90deg, var(--neon-pink), var(--neon-orange));
}

.cyber-card.upload :deep(.progress-glow) {
  background: linear-gradient(90deg, var(--neon-pink), var(--neon-orange));
}

.cyber-card.download :deep(.card-value) {
  color: var(--neon-cyan);
  text-shadow: var(--glow-cyan);
}

.cyber-card.download :deep(.progress-fill) {
  background: linear-gradient(90deg, var(--neon-cyan), var(--neon-blue));
}

.cyber-card.download :deep(.progress-glow) {
  background: linear-gradient(90deg, var(--neon-cyan), var(--neon-blue));
}

.cyber-card.traffic :deep(.card-value) {
  color: var(--neon-green);
  text-shadow: 0 0 20px rgba(0, 255, 136, 0.5);
}

.cyber-card.uptime :deep(.card-value) {
  color: var(--neon-blue);
  text-shadow: var(--glow-blue);
}

.cyber-card.memory :deep(.card-value) {
  color: var(--neon-orange);
  text-shadow: 0 0 20px rgba(255, 107, 53, 0.5);
}

.cyber-card.connections :deep(.card-value) {
  color: var(--neon-green);
  text-shadow: 0 0 20px rgba(0, 255, 136, 0.5);
}

@media (max-width: 640px) {
  .cyber-card {
    padding: 1rem;
  }

  .card-value {
    font-size: var(--font-size-xl);
  }
}

@media (prefers-reduced-motion: reduce) {
  .cyber-card {
    transition: none;
  }

  .cyber-card:hover {
    transform: none;
  }

  .cyber-card::before {
    transition: none;
  }

  .progress-fill {
    transition: none;
  }
}
</style>
