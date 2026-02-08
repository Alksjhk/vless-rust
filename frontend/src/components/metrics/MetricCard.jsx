import { Card, CardContent, CardHeader, CardTitle } from '../ui/card'
import { cn } from '../../utils/cn'

export default function MetricCard({
  title,
  value,
  icon: Icon,
  description,
  trend,
  className,
}) {
  return (
    <Card className={cn('hover:shadow-md transition-shadow', className)}>
      <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
        <CardTitle className="text-sm font-medium text-muted-foreground">
          {title}
        </CardTitle>
        {Icon && <Icon className="h-4 w-4 text-muted-foreground" />}
      </CardHeader>
      <CardContent>
        <div className="text-2xl font-bold text-foreground">{value}</div>
        {description && (
          <p className="text-xs text-muted-foreground mt-1">{description}</p>
        )}
        {trend && (
          <div className="mt-2 flex items-center gap-1">
            <trend.icon className={cn('h-3 w-3', trend.color)} />
            <span className={cn('text-xs font-medium', trend.color)}>
              {trend.value}
            </span>
          </div>
        )}
      </CardContent>
    </Card>
  )
}
