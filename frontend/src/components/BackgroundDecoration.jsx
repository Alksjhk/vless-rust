/**
 * 背景装饰组件
 * 添加动态的背景效果，增强视觉层次
 */
import { clsx } from 'clsx'

/**
 * 渐变光晕装饰
 */
export function GradientOrbs({ className = '' }) {
  return (
    <div className={clsx('fixed inset-0 overflow-hidden pointer-events-none -z-10', className)}>
      {/* 大光晕 */}
      <div className="absolute top-0 left-1/4 w-96 h-96 bg-blue-400/20 dark:bg-blue-600/10 rounded-full blur-3xl animate-float" />
      <div className="absolute bottom-0 right-1/4 w-96 h-96 bg-cyan-400/20 dark:bg-cyan-600/10 rounded-full blur-3xl animate-float" style={{ animationDelay: '2s' }} />

      {/* 小光晕 */}
      <div className="absolute top-1/2 left-1/2 w-64 h-64 bg-purple-400/10 dark:bg-purple-600/5 rounded-full blur-3xl animate-float" style={{ animationDelay: '4s' }} />
    </div>
  )
}

/**
 * 网格背景装饰
 */
export function GridPattern({ className = '' }) {
  return (
    <div
      className={clsx('fixed inset-0 pointer-events-none -z-10', className)}
      style={{
        backgroundImage: `
          linear-gradient(to right, rgba(0,0,0,0.05) 1px, transparent 1px),
          linear-gradient(to bottom, rgba(0,0,0,0.05) 1px, transparent 1px)
        `,
        backgroundSize: '50px 50px',
      }}
    />
  )
}

/**
 * 动态粒子装饰（简化版）
 */
export function ParticleDots({ className = '', count = 20 }) {
  const dots = Array.from({ length: count })

  return (
    <div className={clsx('fixed inset-0 overflow-hidden pointer-events-none -z-10', className)}>
      {dots.map((_, i) => (
        <div
          key={i}
          className="absolute w-1 h-1 bg-blue-400/30 dark:bg-blue-600/20 rounded-full animate-pulse-slow"
          style={{
            left: `${Math.random() * 100}%`,
            top: `${Math.random() * 100}%`,
            animationDelay: `${Math.random() * 3}s`,
          }}
        />
      ))}
    </div>
  )
}

/**
 * 组合背景装饰
 */
export default function BackgroundDecoration({ style = 'gradient' }) {
  return (
    <>
      {style === 'gradient' && <GradientOrbs />}
      {style === 'grid' && <GridPattern />}
      {style === 'particles' && <ParticleDots />}

      {/* 组合样式 */}
      {style === 'combined' && (
        <>
          <GradientOrbs />
          <div className="fixed inset-0 bg-gradient-to-br from-white/30 to-white/10 dark:from-black/20 dark:to-black/10 pointer-events-none -z-10" />
        </>
      )}
    </>
  )
}
