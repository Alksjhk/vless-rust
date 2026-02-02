import { ref, onMounted, onUnmounted } from 'vue'

const THEME_KEY = 'vless-monitor-theme'

export function useTheme() {
  const theme = ref(localStorage.getItem(THEME_KEY) || 'dark')

  const toggleTheme = () => {
    theme.value = theme.value === 'dark' ? 'light' : 'dark'
    applyTheme(theme.value)
  }

  const applyTheme = (newTheme) => {
    if (newTheme === 'light') {
      document.documentElement.setAttribute('data-theme', 'light')
    } else {
      document.documentElement.removeAttribute('data-theme')
    }
    localStorage.setItem(THEME_KEY, newTheme)
  }

  const isDark = () => theme.value === 'dark'

  onMounted(() => {
    applyTheme(theme.value)
  })

  return {
    theme,
    toggleTheme,
    isDark
  }
}
