import { Wifi, WifiOff } from 'lucide-react'
import { Badge } from '../ui/badge'

export default function Header({ connectionStatus }) {
  const isConnected = connectionStatus === 'connected'

  return (
    <header className="sticky top-0 z-50 w-full border-b border-border bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60">
      <div className="container mx-auto flex h-16 items-center justify-between px-4">
        {/* 左侧：Logo 和标题 */}
        <div className="flex items-center gap-3">
          <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-sky-100 overflow-hidden">
            <img src="/vite.svg" alt="Vite" className="h-6 w-6" />
          </div>
          <div>
            <h1 className="text-xl font-bold text-foreground">VLESS Monitor</h1>
            <p className="text-xs text-muted-foreground">服务器监控面板</p>
          </div>
        </div>

        {/* 右侧：连接状态 */}
        <div className="flex items-center gap-4">
          <Badge variant={isConnected ? 'success' : 'destructive'} className="gap-1.5">
            {isConnected ? (
              <>
                <Wifi className="h-3.5 w-3.5" />
                <span>已连接</span>
              </>
            ) : (
              <>
                <WifiOff className="h-3.5 w-3.5" />
                <span>未连接</span>
              </>
            )}
          </Badge>
        </div>
      </div>
    </header>
  )
}
