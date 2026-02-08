import { memo } from 'react'
import { Card, CardContent, CardHeader, CardTitle } from '../ui/card'
import { Activity, Wifi } from 'lucide-react'
import SpeedChart from './SpeedChart'
import useMonitorStore from '../../store/monitorStore'
import { Badge } from '../ui/badge'

function TrafficChartSection() {
  const { speedHistory, isConnected, isPolling, historyDuration } = useMonitorStore()

  // 连接状态指示器
  const ConnectionStatus = () => {
    if (isConnected) {
      return (
        <Badge variant="outline" className="gap-1.5 border-green-500/50 bg-green-500/10 text-green-700 dark:text-green-400">
          <span className="relative flex h-2 w-2">
            <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-green-400 opacity-75"></span>
            <span className="relative inline-flex h-2 w-2 rounded-full bg-green-500"></span>
          </span>
          WebSocket
        </Badge>
      )
    }

    if (isPolling) {
      return (
        <Badge variant="outline" className="gap-1.5 border-yellow-500/50 bg-yellow-500/10 text-yellow-700 dark:text-yellow-400">
          <Wifi className="h-3 w-3" />
          API 轮询
        </Badge>
      )
    }

    return (
      <Badge variant="outline" className="gap-1.5 border-muted-foreground/50 bg-muted/10">
        <span className="h-2 w-2 rounded-full bg-muted-foreground"></span>
        连接中...
      </Badge>
    )
  }

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center justify-between">
          <CardTitle className="flex items-center gap-2">
            <Activity className="h-5 w-5 text-primary" />
            流量趋势
          </CardTitle>
          <div className="flex items-center gap-3">
            <span className="text-sm text-muted-foreground">
              过去 {Math.floor(historyDuration / 60)} 分钟
            </span>
            <ConnectionStatus />
          </div>
        </div>
      </CardHeader>
      <CardContent className="pt-2">
        <SpeedChart speedHistory={speedHistory} showArea={true} height={500} />
      </CardContent>
    </Card>
  )
}

export default memo(TrafficChartSection)
