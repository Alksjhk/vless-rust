/**
 * 主题切换组件
 * 支持亮色/暗色/跟随系统模式
 */
import { useState, useEffect } from 'react'
import { SunIcon, MoonIcon, ComputerDesktopIcon } from '@heroicons/react/24/outline'

export default function ThemeToggle() {
  const [theme, setTheme] = useState('light')
  const [isOpen, setIsOpen] = useState(false)

  // 初始化主题
  useEffect(() => {
    const savedTheme = localStorage.getItem('theme')
    const systemTheme = window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'

    const initialTheme = savedTheme || systemTheme
    setTheme(initialTheme)
    applyTheme(initialTheme)
  }, [])

  // 应用主题
  const applyTheme = (newTheme) => {
    const root = document.documentElement
    if (newTheme === 'dark') {
      root.classList.add('dark')
    } else {
      root.classList.remove('dark')
    }
  }

  // 切换主题
  const handleThemeChange = (newTheme) => {
    setTheme(newTheme)
    localStorage.setItem('theme', newTheme)
    applyTheme(newTheme)
    setIsOpen(false)
  }

  // 主题选项
  const themes = [
    { value: 'light', icon: SunIcon, label: '亮色模式' },
    { value: 'dark', icon: MoonIcon, label: '深色模式' },
    { value: 'system', icon: ComputerDesktopIcon, label: '跟随系统' },
  ]

  // 当前主题图标
  const CurrentIcon = theme === 'dark' ? MoonIcon : SunIcon

  return (
    <div className="relative">
      {/* 主按钮 */}
      <button
        onClick={() => setIsOpen(!isOpen)}
        className="p-2 rounded-xl glass-card btn-hover"
        aria-label="切换主题"
      >
        <CurrentIcon className="w-5 h-5 text-gray-700 dark:text-gray-200" />
      </button>

      {/* 下拉菜单 */}
      {isOpen && (
        <>
          {/* 遮罩层 */}
          <div
            className="fixed inset-0 z-10"
            onClick={() => setIsOpen(false)}
          />

          {/* 菜单 */}
          <div className="absolute right-0 mt-2 w-48 glass-primary py-2 z-20 animate-scale-in">
            {themes.map(({ value, icon: Icon, label }) => (
              <button
                key={value}
                onClick={() => handleThemeChange(value)}
                className={`w-full px-4 py-3 flex items-center gap-3 transition-colors ${
                  theme === value
                    ? 'bg-blue-50 dark:bg-blue-900/30 text-blue-600 dark:text-blue-400'
                    : 'text-gray-700 dark:text-gray-200 hover:bg-gray-50 dark:hover:bg-gray-800/50'
                }`}
              >
                <Icon className="w-5 h-5" />
                <span className="text-sm font-medium">{label}</span>
                {theme === value && (
                  <span className="ml-auto w-2 h-2 rounded-full bg-blue-500" />
                )}
              </button>
            ))}
          </div>
        </>
      )}
    </div>
  )
}
