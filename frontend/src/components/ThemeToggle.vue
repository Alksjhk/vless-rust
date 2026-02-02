<template>
  <button
    class="theme-toggle"
    @click="toggleTheme"
    :aria-label="isDark() ? '切换到浅色模式' : '切换到深色模式'"
  >
    <span class="toggle-icon">{{ isDark() ? '◐' : '◑' }}</span>
    <span class="toggle-label">{{ isDark() ? '深色' : '浅色' }}</span>
  </button>
</template>

<script setup>
import { useTheme } from '../composables/useTheme'

const { toggleTheme, isDark } = useTheme()
</script>

<style scoped>
.theme-toggle {
  position: relative;
  background: var(--bg-glass);
  border: 1px solid var(--border-color);
  color: var(--text-primary);
  padding: 0.5rem 1rem;
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.3s ease;
  font-size: var(--font-size-sm);
  display: flex;
  align-items: center;
  gap: 0.5rem;
  backdrop-filter: blur(10px);
  overflow: hidden;
  text-transform: uppercase;
  letter-spacing: 1px;
  font-weight: 600;
}

.theme-toggle::before {
  content: '';
  position: absolute;
  top: 0;
  left: -100%;
  width: 100%;
  height: 100%;
  background: linear-gradient(
    90deg,
    transparent,
    rgba(0, 245, 255, 0.2),
    transparent
  );
  transition: left 0.5s ease;
}

.theme-toggle:hover::before {
  left: 100%;
}

.theme-toggle:hover {
  border-color: var(--neon-cyan);
  box-shadow: var(--glow-cyan);
  transform: translateY(-1px);
}

.theme-toggle:focus {
  outline: none;
  border-color: var(--neon-cyan);
  box-shadow: 0 0 0 3px rgba(0, 245, 255, 0.2);
}

.toggle-icon {
  font-size: 1.5em;
  color: var(--neon-cyan);
  text-shadow: var(--glow-cyan);
  display: flex;
  align-items: center;
  justify-content: center;
}

.toggle-label {
  font-size: var(--font-size-xs);
}

@media (max-width: 640px) {
  .theme-toggle {
    padding: 0.4rem 0.8rem;
  }

  .toggle-label {
    display: none;
  }
}

@media (prefers-reduced-motion: reduce) {
  .theme-toggle {
    transition: none;
  }

  .theme-toggle::before {
    transition: none;
  }

  .theme-toggle:hover {
    transform: none;
  }
}
</style>
