import { Card, CardContent, CardHeader, CardTitle } from '../ui/card'
import { Globe } from 'lucide-react'
import useMonitorStore from '../../store/monitorStore'

export default function SystemPanel() {
  const { publicIp } = useMonitorStore()

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Globe className="h-5 w-5 text-primary" />
          系统信息
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-6">
        {/* 公网IP */}
        <div className="space-y-2">
          <div className="flex items-center gap-2 text-sm">
            <Globe className="h-4 w-4 text-muted-foreground" />
            <span className="font-medium">公网IP</span>
          </div>
          <p className="text-sm font-semibold pl-6">{publicIp}</p>
        </div>
      </CardContent>
    </Card>
  )
}
