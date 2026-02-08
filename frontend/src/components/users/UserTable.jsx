import { Card, CardContent, CardHeader, CardTitle } from '../ui/card'
import { Users } from 'lucide-react'
import useMonitorStore from '../../store/monitorStore'

export default function UserTable() {
  const { users } = useMonitorStore()

  if (!users || users.length === 0) {
    return (
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Users className="h-5 w-5 text-primary" />
            用户统计
          </CardTitle>
        </CardHeader>
        <CardContent>
          <div className="text-center py-8 text-muted-foreground">
            暂无用户数据
          </div>
        </CardContent>
      </Card>
    )
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Users className="h-5 w-5 text-primary" />
          用户统计
        </CardTitle>
      </CardHeader>
      <CardContent>
        <div className="overflow-x-auto">
          <table className="w-full">
            <thead>
              <tr className="border-b border-border">
                <th className="pb-3 text-left text-sm font-medium text-muted-foreground">
                  用户ID
                </th>
                <th className="pb-3 text-left text-sm font-medium text-muted-foreground">
                  邮箱
                </th>
                <th className="pb-3 text-right text-sm font-medium text-muted-foreground">
                  上传速度
                </th>
                <th className="pb-3 text-right text-sm font-medium text-muted-foreground">
                  下载速度
                </th>
                <th className="pb-3 text-right text-sm font-medium text-muted-foreground">
                  总流量
                </th>
                <th className="pb-3 text-right text-sm font-medium text-muted-foreground">
                  连接数
                </th>
              </tr>
            </thead>
            <tbody>
              {users.map((user, index) => (
                <tr
                  key={user.uuid || index}
                  className="border-b border-border last:border-0 hover:bg-muted/50 transition-colors"
                >
                  <td className="py-3 text-sm">
                    <code className="text-xs bg-muted px-2 py-1 rounded">
                      {user.uuid.slice(0, 8)}...
                    </code>
                  </td>
                  <td className="py-3 text-sm text-foreground">
                    {user.email || '-'}
                  </td>
                  <td className="py-3 text-sm text-right font-medium text-blue-600">
                    {user.upload_speed || '0 B/s'}
                  </td>
                  <td className="py-3 text-sm text-right font-medium text-green-600">
                    {user.download_speed || '0 B/s'}
                  </td>
                  <td className="py-3 text-sm text-right font-medium">
                    {user.total_traffic || '0 B'}
                  </td>
                  <td className="py-3 text-sm text-right">
                    <span className="inline-flex items-center px-2 py-1 rounded-md bg-secondary text-secondary-foreground text-xs font-medium">
                      {user.active_connections || 0}
                    </span>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </CardContent>
    </Card>
  )
}
