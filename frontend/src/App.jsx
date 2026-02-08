import React, { Suspense, lazy } from 'react'
import useMonitorStore from './store/monitorStore'
import DashboardLayout from './components/layouts/DashboardLayout'
import MetricsGrid from './components/layouts/MetricsGrid'
import { Card, CardContent } from './components/ui/card'

// 懒加载组件
const SpeedMetric = lazy(() => import('./components/metrics/SpeedMetric'))
const TrafficMetric = lazy(() => import('./components/metrics/TrafficMetric'))
const ConnectionsMetric = lazy(() => import('./components/metrics/ConnectionsMetric'))
const UptimeMetric = lazy(() => import('./components/metrics/UptimeMetric'))
const MemoryMetric = lazy(() => import('./components/metrics/MemoryMetric'))
const TrafficChartSection = lazy(() => import('./components/charts/TrafficChartSection'))
const SystemPanel = lazy(() => import('./components/system/SystemPanel'))
const UserStatsSection = lazy(() => import('./components/users/UserStatsSection'))

// 骨架屏组件
function MetricSkeleton() {
  return (
    <Card>
      <CardContent className="p-6">
        <div className="space-y-3">
          <div className="h-4 bg-muted rounded w-20" />
          <div className="h-8 bg-muted rounded w-32" />
        </div>
      </CardContent>
    </Card>
  )
}

function ChartSkeleton() {
  return (
    <Card>
      <CardContent className="p-6">
        <div className="h-64 bg-muted rounded animate-pulse" />
      </CardContent>
    </Card>
  )
}

function App() {
  const { connect, disconnect } = useMonitorStore()

  React.useEffect(() => {
    // 连接 WebSocket
    connect()

    return () => {
      disconnect()
    }
  }, [connect, disconnect])

  return (
    <DashboardLayout>
      {/* 指标卡片网格 */}
      <section className="mb-6">
        <Suspense fallback={<MetricSkeleton />}>
          <MetricsGrid>
            <SpeedMetric />
            <TrafficMetric />
            <ConnectionsMetric />
            <UptimeMetric />
            <MemoryMetric />
          </MetricsGrid>
        </Suspense>
      </section>

      {/* 主内容区域：图表和系统信息 */}
      <section className="grid gap-6 lg:grid-cols-6 mb-6">
        {/* 流量趋势图 */}
        <div className="lg:col-span-5">
          <Suspense fallback={<ChartSkeleton />}>
            <TrafficChartSection />
          </Suspense>
        </div>

        {/* 系统信息面板 */}
        <div className="lg:col-span-1">
          <Suspense fallback={<MetricSkeleton />}>
            <SystemPanel />
          </Suspense>
        </div>
      </section>

      {/* 用户统计表格 */}
      <section>
        <Suspense fallback={<ChartSkeleton />}>
          <UserStatsSection />
        </Suspense>
      </section>
    </DashboardLayout>
  )
}

export default App
