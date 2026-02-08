/**
 * 速度趋势图组件
 * 使用 Victory 图表库实现，支持面积图和折线图
 */
import { memo, useMemo, useState, useRef, useEffect } from 'react'
import {
  VictoryChart,
  VictoryArea,
  VictoryAxis,
  VictoryTooltip,
  VictoryVoronoiContainer,
  VictoryLegend,
  VictoryTheme
} from 'victory'
import { parseSpeedString } from '../../utils/formatters'

function SpeedChart({ speedHistory, showArea = true, height = 400 }) {
  const [chartWidth, setChartWidth] = useState(600)
  const containerRef = useRef(null)

  // 监听容器宽度变化
  useEffect(() => {
    const container = containerRef.current
    if (!container) return

    const updateWidth = () => {
      const width = container.getBoundingClientRect().width
      setChartWidth(width)
    }

    // 初始化宽度
    updateWidth()

    // 监听窗口大小变化
    const resizeObserver = new ResizeObserver(updateWidth)
    resizeObserver.observe(container)

    return () => {
      resizeObserver.disconnect()
    }
  }, [])

  // 转换数据格式
  const chartData = useMemo(() => {
    if (!speedHistory || speedHistory.length === 0) {
      return []
    }

    return speedHistory.map((item, index) => {
      const timestamp = parseInt(item.timestamp)
      const date = new Date(timestamp * 1000)
      
      return {
        x: index,
        time: date.toLocaleTimeString('zh-CN', {
          hour: '2-digit',
          minute: '2-digit',
          second: '2-digit'
        }),
        upload: parseSpeedString(item.upload_speed || '0 B/s'),
        download: parseSpeedString(item.download_speed || '0 B/s'),
        timestamp
      }
    })
  }, [speedHistory])

  // 计算 Y 轴最大值（动态调整）
  // 默认 200 KB/s，超过后动态变化
  const DEFAULT_MAX_Y = 200 / 1024 // 200 KB/s = 0.195 MB/s
  const maxY = useMemo(() => {
    if (chartData.length === 0) return DEFAULT_MAX_Y

    const maxUpload = Math.max(...chartData.map(d => d.upload))
    const maxDownload = Math.max(...chartData.map(d => d.download))
    const max = Math.max(maxUpload, maxDownload)

    // 如果最大值小于默认值，使用默认值
    if (max < DEFAULT_MAX_Y) return DEFAULT_MAX_Y

    // 向上取整到合适的刻度
    if (max < 0.1) return 0.1
    if (max < 1) return Math.ceil(max * 10) / 10
    if (max < 10) return Math.ceil(max)
    return Math.ceil(max / 10) * 10
  }, [chartData])

  // 格式化速度显示
  const formatSpeed = (value) => {
    if (value < 0.001) return '0 KB/s'
    if (value < 1) return `${(value * 1024).toFixed(0)} KB/s`
    return `${value.toFixed(2)} MB/s`
  }

  // X 轴刻度（显示10个均匀分布的时间点）
  const xTickValues = useMemo(() => {
    if (chartData.length === 0) return []
    const len = chartData.length
    if (len <= 10) return chartData.map((_, i) => i)

    // 生成10个均匀分布的索引
    const ticks = []
    for (let i = 0; i < 10; i++) {
      const index = Math.floor((i / 9) * (len - 1))
      ticks.push(index)
    }
    return ticks
  }, [chartData])

  if (chartData.length === 0) {
    return (
      <div className="flex items-center justify-center h-64 text-muted-foreground">
        暂无数据
      </div>
    )
  }

  return (
    <div ref={containerRef} className="w-full" style={{ height }}>
      <VictoryChart
        theme={VictoryTheme.material}
        width={chartWidth}
        height={height}
        padding={{ top: 20, right: 40, bottom: 60, left: 60 }}
        containerComponent={
          <VictoryVoronoiContainer
            voronoiDimension="x"
            labelComponent={
              <VictoryTooltip
                cornerRadius={8}
                flyoutStyle={{
                  fill: 'hsl(var(--card))',
                  stroke: 'hsl(var(--border))',
                  strokeWidth: 1
                }}
                style={{
                  fill: 'hsl(var(--foreground))',
                  fontSize: 12
                }}
                flyoutPadding={{ top: 8, bottom: 8, left: 12, right: 12 }}
              />
            }
            labels={({ datum }) => {
              const point = chartData[datum.x]
              if (!point) return ''
              
              const isUpload = datum.childName?.includes('upload')
              const speed = isUpload ? point.upload : point.download
              const label = isUpload ? '上传' : '下载'
              
              return `${point.time}\n${label}: ${formatSpeed(speed)}`
            }}
          />
        }
        domain={{ y: [0, maxY] }}
      >
        <defs>
          <linearGradient id="uploadGradient" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stopColor="#3b82f6" stopOpacity={0.4} />
            <stop offset="100%" stopColor="#3b82f6" stopOpacity={0.05} />
          </linearGradient>
          <linearGradient id="downloadGradient" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stopColor="#10b981" stopOpacity={0.4} />
            <stop offset="100%" stopColor="#10b981" stopOpacity={0.05} />
          </linearGradient>
        </defs>

        {/* X 轴 */}
        <VictoryAxis
          tickValues={xTickValues}
          tickFormat={(index) => chartData[index]?.time || ''}
          style={{
            axis: { stroke: 'hsl(var(--border))' },
            tickLabels: {
              fontSize: 11,
              fill: 'hsl(var(--muted-foreground))',
              angle: -45,
              textAnchor: 'end',
              padding: 5
            }
          }}
        />

        {/* Y 轴 */}
        <VictoryAxis
          dependentAxis
          tickFormat={formatSpeed}
          style={{
            axis: { stroke: 'hsl(var(--border))' },
            tickLabels: {
              fontSize: 11,
              fill: 'hsl(var(--muted-foreground))'
            },
            grid: {
              stroke: 'hsl(var(--border))',
              strokeDasharray: '4,4',
              strokeOpacity: 0.3
            }
          }}
        />

        {/* 上传区域 */}
        <VictoryArea
          name="upload-area"
          data={chartData}
          x="x"
          y="upload"
          interpolation="monotoneX"
          style={{
            data: {
              fill: showArea ? 'url(#uploadGradient)' : 'none',
              stroke: '#3b82f6',
              strokeWidth: 2
            }
          }}
        />

        {/* 下载区域 */}
        <VictoryArea
          name="download-area"
          data={chartData}
          x="x"
          y="download"
          interpolation="monotoneX"
          style={{
            data: {
              fill: showArea ? 'url(#downloadGradient)' : 'none',
              stroke: '#10b981',
              strokeWidth: 2
            }
          }}
        />

        {/* 图例 */}
        <VictoryLegend
          x={60}
          y={10}
          orientation="horizontal"
          gutter={24}
          style={{
            labels: {
              fontSize: 12,
              fill: 'hsl(var(--foreground))'
            }
          }}
          data={[
            { name: '上传', symbol: { fill: '#3b82f6', type: 'square' } },
            { name: '下载', symbol: { fill: '#10b981', type: 'square' } }
          ]}
        />
      </VictoryChart>
    </div>
  )
}

export default memo(SpeedChart)
