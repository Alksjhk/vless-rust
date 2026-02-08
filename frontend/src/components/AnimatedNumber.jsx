/**
 * 数字滚动动画组件
 * 使用 CSS transform 实现平滑的数字变化效果
 */
import { useState, useEffect, useRef } from 'react'
import { clsx } from 'clsx'

export default function AnimatedNumber({
  value = 0,
  duration = 500,
  decimals = 0,
  className = '',
}) {
  const [displayValue, setDisplayValue] = useState(value)
  const [isAnimating, setIsAnimating] = useState(false)
  const timeoutRef = useRef(null)
  const previousValueRef = useRef(value)

  useEffect(() => {
    // 如果值没有变化，不执行动画
    if (previousValueRef.current === value) return

    // 清除之前的动画
    if (timeoutRef.current) {
      clearTimeout(timeoutRef.current)
    }

    setIsAnimating(true)

    // 简单的数字插值动画
    const startValue = previousValueRef.current
    const endValue = value
    const startTime = Date.now()

    const animate = () => {
      const elapsed = Date.now() - startTime
      const progress = Math.min(elapsed / duration, 1)

      // 使用 easeOutQuart 缓动函数
      const easeOutQuart = (t) => 1 - Math.pow(1 - t, 4)
      const easedProgress = easeOutQuart(progress)

      const currentValue = startValue + (endValue - startValue) * easedProgress
      setDisplayValue(currentValue)

      if (progress < 1) {
        timeoutRef.current = requestAnimationFrame(animate)
      } else {
        setIsAnimating(false)
        previousValueRef.current = value
      }
    }

    timeoutRef.current = requestAnimationFrame(animate)

    return () => {
      if (timeoutRef.current) {
        cancelAnimationFrame(timeoutRef.current)
      }
    }
  }, [value, duration])

  // 格式化数值
  const formatValue = (val) => {
    if (decimals === 0) {
      return Math.round(val).toLocaleString()
    }
    return val.toFixed(decimals)
  }

  return (
    <span
      className={clsx(
        'inline-block transition-transform duration-300',
        isAnimating && 'scale-110',
        className
      )}
    >
      {formatValue(displayValue)}
    </span>
  )
}

/**
 * 带单位的数字动画组件
 */
export function AnimatedNumberUnit({
  value = 0,
  unit = '',
  duration = 500,
  decimals = 0,
  className = '',
}) {
  return (
    <div className={className}>
      <AnimatedNumber value={value} duration={duration} decimals={decimals} />
      {unit && <span className="ml-1 text-sm text-gray-500 dark:text-gray-400">{unit}</span>}
    </div>
  )
}

/**
 * 速度数字动画组件（带方向图标）
 */
export function AnimatedSpeed({
  value = 0,
  unit = 'MB/s',
  direction = 'up', // 'up' | 'down'
  duration = 500,
  decimals = 2,
}) {
  const Icon = direction === 'up'
    ? () => <span className="text-blue-500">↑</span>
    : () => <span className="text-green-500">↓</span>

  return (
    <div className="flex items-center gap-2">
      <span className="text-lg">
        <Icon />
      </span>
      <AnimatedNumber
        value={value}
        duration={duration}
        decimals={decimals}
        className="text-xl font-semibold"
      />
      <span className="text-sm text-gray-500 dark:text-gray-400">{unit}</span>
    </div>
  )
}
