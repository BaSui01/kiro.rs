import { Card, CardContent, CardHeader, CardTitle } from './ui/card'
import { Badge } from './ui/badge'
import { Switch } from './ui/switch'
import { Button } from './ui/button'
import type { PoolStatusItem, SchedulingMode } from '../types/api'
import { Trash2, Settings, Users, Shuffle, ArrowDownToLine, Globe, Zap } from 'lucide-react'

interface PoolCardProps {
  pool: PoolStatusItem
  onToggleEnabled: (poolId: string, enabled: boolean) => void
  onEdit: (pool: PoolStatusItem) => void
  onDelete: (poolId: string) => void
}

const schedulingModeConfig: Record<SchedulingMode, { label: string; icon: React.ReactNode; color: string }> = {
  round_robin: {
    label: '轮询',
    icon: <Shuffle className="h-3 w-3" />,
    color: 'bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-400'
  },
  priority_fill: {
    label: '优先填充',
    icon: <ArrowDownToLine className="h-3 w-3" />,
    color: 'bg-orange-100 text-orange-700 dark:bg-orange-900/30 dark:text-orange-400'
  },
}

export function PoolCard({ pool, onToggleEnabled, onEdit, onDelete }: PoolCardProps) {
  const isDefault = pool.id === 'default'
  const modeConfig = schedulingModeConfig[pool.schedulingMode]

  return (
    <Card className={`relative overflow-hidden transition-all duration-200 hover:shadow-lg ${!pool.enabled ? 'opacity-60 grayscale' : ''}`}>
      {/* 顶部装饰条 */}
      <div className={`absolute top-0 left-0 right-0 h-1 ${isDefault ? 'bg-gradient-to-r from-cyan-500 to-blue-600' : 'bg-gradient-to-r from-gray-300 to-gray-400 dark:from-gray-600 dark:to-gray-700'}`} />

      <CardHeader className="pb-2 pt-4">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <CardTitle className="text-lg">{pool.name}</CardTitle>
            {isDefault && (
              <Badge className="bg-gradient-to-r from-cyan-500 to-blue-600 text-white border-0 text-xs">
                默认
              </Badge>
            )}
          </div>
          <div className="flex items-center gap-2">
            <Switch
              checked={pool.enabled}
              onCheckedChange={(checked) => onToggleEnabled(pool.id, checked)}
              disabled={isDefault}
            />
          </div>
        </div>
        {pool.description && (
          <p className="text-sm text-muted-foreground mt-1">{pool.description}</p>
        )}
      </CardHeader>
      <CardContent className="space-y-3">
        {/* 池 ID */}
        <div className="flex items-center justify-between text-sm">
          <span className="text-muted-foreground">池 ID</span>
          <code className="bg-muted px-2 py-0.5 rounded text-xs font-mono">{pool.id}</code>
        </div>

        {/* 调度模式 */}
        <div className="flex items-center justify-between text-sm">
          <span className="text-muted-foreground flex items-center gap-1">
            <Zap className="h-3 w-3" />
            调度模式
          </span>
          <Badge variant="outline" className={modeConfig.color}>
            {modeConfig.icon}
            <span className="ml-1">{modeConfig.label}</span>
          </Badge>
        </div>

        {/* 凭据统计 */}
        <div className="flex items-center justify-between text-sm">
          <span className="text-muted-foreground flex items-center gap-1">
            <Users className="h-3 w-3" />
            凭据
          </span>
          <span className="font-medium">
            <span className="text-green-600">{pool.availableCredentials}</span>
            <span className="text-muted-foreground"> / {pool.totalCredentials}</span>
            <span className="text-xs text-muted-foreground ml-1">可用</span>
          </span>
        </div>

        {/* 代理状态 */}
        <div className="flex items-center justify-between text-sm">
          <span className="text-muted-foreground flex items-center gap-1">
            <Globe className="h-3 w-3" />
            代理
          </span>
          <Badge variant={pool.hasProxy ? 'default' : 'secondary'} className={pool.hasProxy ? 'bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400' : ''}>
            {pool.hasProxy ? '已配置' : '未配置'}
          </Badge>
        </div>

        {/* 会话缓存 */}
        <div className="flex items-center justify-between text-sm">
          <span className="text-muted-foreground">会话缓存</span>
          <span className="font-medium">{pool.sessionCacheSize}</span>
        </div>

        {/* 操作按钮 */}
        <div className="flex gap-2 pt-3 border-t">
          <Button
            variant="outline"
            size="sm"
            className="flex-1"
            onClick={() => onEdit(pool)}
          >
            <Settings className="h-4 w-4 mr-1" />
            编辑
          </Button>
          {!isDefault && (
            <Button
              variant="outline"
              size="sm"
              className="text-destructive hover:text-destructive hover:bg-destructive/10"
              onClick={() => onDelete(pool.id)}
            >
              <Trash2 className="h-4 w-4" />
            </Button>
          )}
        </div>
      </CardContent>
    </Card>
  )
}
