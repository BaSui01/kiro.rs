import {
  Plus,
  Upload,
  Shuffle,
  ArrowDownToLine,
  ChevronDown,
  ChevronRight,
} from 'lucide-react'
import { Card } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { CredentialCard } from '@/components/credential-card'
import type { PoolStatusItem, SchedulingMode, CredentialStatusItem } from '@/types/api'

export interface PoolItemProps {
  pool: PoolStatusItem
  expanded: boolean
  onToggleExpand: () => void
  onEdit: () => void
  onDelete: () => void
  onToggleEnabled: (enabled: boolean) => void
  credentials: CredentialStatusItem[]
  onViewBalance: (id: number) => void
  onAddCredential: () => void
  onImportCredentials: () => void
}

export function PoolItem({
  pool,
  expanded,
  onToggleExpand,
  onEdit,
  onDelete,
  onToggleEnabled,
  credentials,
  onViewBalance,
  onAddCredential,
  onImportCredentials,
}: PoolItemProps) {
  const isDefault = pool.id === 'default'
  const schedulingModeLabel = pool.schedulingMode === 'round_robin' ? '轮询' : '优先填充'
  const SchedulingModeIcon = pool.schedulingMode === 'round_robin' ? Shuffle : ArrowDownToLine

  return (
    <Card className={`${!pool.enabled ? 'opacity-60' : ''}`}>
      {/* 池头部 - 可点击展开/折叠 */}
      <div
        className="flex items-center justify-between p-4 cursor-pointer hover:bg-muted/50 transition-colors"
        onClick={onToggleExpand}
      >
        <div className="flex items-center gap-3">
          {/* 展开/折叠图标 */}
          <div className="text-muted-foreground">
            {expanded ? <ChevronDown className="h-5 w-5" /> : <ChevronRight className="h-5 w-5" />}
          </div>

          {/* 池信息 */}
          <div>
            <div className="flex items-center gap-2">
              <span className="font-semibold">{pool.name}</span>
              <Badge variant="outline" className="text-xs">
                {pool.id}
              </Badge>
              {isDefault && (
                <Badge variant="secondary" className="text-xs">
                  默认
                </Badge>
              )}
              {!pool.enabled && (
                <Badge variant="destructive" className="text-xs">
                  已禁用
                </Badge>
              )}
            </div>
            <div className="flex items-center gap-4 text-sm text-muted-foreground mt-1">
              <span className="flex items-center gap-1">
                <SchedulingModeIcon className="h-3 w-3" />
                {schedulingModeLabel}
              </span>
              <span>
                {pool.availableCredentials}/{pool.totalCredentials} 可用
              </span>
              {pool.hasProxy && (
                <Badge variant="outline" className="text-xs">
                  代理
                </Badge>
              )}
            </div>
          </div>
        </div>

        {/* 操作按钮 */}
        <div className="flex items-center gap-2" onClick={(e) => e.stopPropagation()}>
          <Button variant="ghost" size="sm" onClick={onEdit}>
            编辑
          </Button>
          {!isDefault && (
            <Button
              variant="ghost"
              size="sm"
              onClick={() => onToggleEnabled(!pool.enabled)}
            >
              {pool.enabled ? '禁用' : '启用'}
            </Button>
          )}
          {!isDefault && pool.totalCredentials === 0 && (
            <Button variant="ghost" size="sm" className="text-destructive" onClick={onDelete}>
              删除
            </Button>
          )}
        </div>
      </div>

      {/* 展开的内容 - 凭据列表 */}
      {expanded && (
        <div className="border-t px-4 py-4 bg-muted/30">
          {isDefault ? (
            <>
              {/* 默认池显示凭据列表 */}
              <div className="flex items-center justify-between mb-4">
                <span className="text-sm font-medium">凭据列表</span>
                <div className="flex gap-2">
                  <Button onClick={onImportCredentials} size="sm" variant="outline">
                    <Upload className="h-4 w-4 mr-1" />
                    导入
                  </Button>
                  <Button onClick={onAddCredential} size="sm">
                    <Plus className="h-4 w-4 mr-1" />
                    添加
                  </Button>
                </div>
              </div>

              {credentials.length === 0 ? (
                <div className="text-center py-8 text-muted-foreground">
                  暂无凭据，点击"添加"或"导入"添加凭据
                </div>
              ) : (
                <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
                  {credentials.map((credential) => (
                    <CredentialCard
                      key={credential.id}
                      credential={credential}
                      onViewBalance={onViewBalance}
                      schedulingMode={pool.schedulingMode as SchedulingMode}
                    />
                  ))}
                </div>
              )}
            </>
          ) : (
            /* 非默认池显示统计信息 */
            <div className="text-center py-8">
              <div className="grid grid-cols-3 gap-4 max-w-md mx-auto">
                <div>
                  <div className="text-2xl font-bold">{pool.totalCredentials}</div>
                  <div className="text-xs text-muted-foreground">总凭据</div>
                </div>
                <div>
                  <div className="text-2xl font-bold text-green-600">{pool.availableCredentials}</div>
                  <div className="text-xs text-muted-foreground">可用</div>
                </div>
                <div>
                  <div className="text-2xl font-bold text-blue-600">{pool.sessionCacheSize}</div>
                  <div className="text-xs text-muted-foreground">会话缓存</div>
                </div>
              </div>
              <p className="text-sm text-muted-foreground mt-4">
                提示：将凭据分配到此池后，可在此查看详情
              </p>
            </div>
          )}
        </div>
      )}
    </Card>
  )
}
