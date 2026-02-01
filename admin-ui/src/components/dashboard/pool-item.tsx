import {
  Plus,
  Upload,
  Shuffle,
  ArrowDownToLine,
  ChevronDown,
  ChevronRight,
  Shield,
  Wifi,
  Users,
  Key,
  Sparkles,
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
    <Card className={`overflow-hidden border-0 shadow-sm hover:shadow-md transition-all duration-300 ${!pool.enabled ? 'opacity-60' : ''}`}>
      <div
        className="flex items-center justify-between p-5 cursor-pointer hover:bg-muted/30 transition-colors"
        onClick={onToggleExpand}
      >
        <div className="flex items-center gap-4">
          <div className={`flex items-center justify-center w-12 h-12 rounded-xl transition-all ${
            expanded 
              ? 'bg-gradient-to-br from-indigo-500 to-purple-600 shadow-lg shadow-indigo-500/25' 
              : 'bg-muted'
          }`}>
            {expanded 
              ? <ChevronDown className="h-5 w-5 text-white" /> 
              : <ChevronRight className="h-5 w-5 text-muted-foreground" />
            }
          </div>

          <div>
            <div className="flex items-center gap-2 mb-1">
              <span className="font-semibold text-lg">{pool.name}</span>
              <Badge variant="outline" className="text-xs font-mono">
                {pool.id}
              </Badge>
              {isDefault && (
                <Badge className="text-xs bg-gradient-to-r from-amber-500 to-orange-500 border-0">
                  <Sparkles className="h-3 w-3 mr-1" />
                  默认
                </Badge>
              )}
              {!pool.enabled && (
                <Badge variant="destructive" className="text-xs">
                  已禁用
                </Badge>
              )}
            </div>
            <div className="flex items-center gap-4 text-sm text-muted-foreground">
              <span className="flex items-center gap-1.5 px-2 py-0.5 rounded-full bg-muted">
                <SchedulingModeIcon className="h-3 w-3" />
                {schedulingModeLabel}
              </span>
              <span className="flex items-center gap-1.5">
                <Key className="h-3 w-3" />
                {pool.availableCredentials}/{pool.totalCredentials} 可用
              </span>
              {pool.hasProxy && (
                <span className="flex items-center gap-1.5 px-2 py-0.5 rounded-full bg-blue-500/10 text-blue-600 dark:text-blue-400">
                  <Wifi className="h-3 w-3" />
                  代理
                </span>
              )}
              {pool.sessionCacheSize > 0 && (
                <span className="flex items-center gap-1.5 px-2 py-0.5 rounded-full bg-purple-500/10 text-purple-600 dark:text-purple-400">
                  <Users className="h-3 w-3" />
                  {pool.sessionCacheSize} 会话
                </span>
              )}
            </div>
          </div>
        </div>

        <div className="flex items-center gap-2" onClick={(e) => e.stopPropagation()}>
          <Button variant="ghost" size="sm" className="rounded-lg" onClick={onEdit}>
            编辑
          </Button>
          {!isDefault && (
            <Button
              variant="ghost"
              size="sm"
              className="rounded-lg"
              onClick={() => onToggleEnabled(!pool.enabled)}
            >
              {pool.enabled ? '禁用' : '启用'}
            </Button>
          )}
          {!isDefault && pool.totalCredentials === 0 && (
            <Button variant="ghost" size="sm" className="rounded-lg text-destructive hover:text-destructive" onClick={onDelete}>
              删除
            </Button>
          )}
        </div>
      </div>

      {expanded && (
        <div className="border-t bg-gradient-to-b from-muted/50 to-muted/20 px-5 py-5">
          {isDefault ? (
            <>
              <div className="flex items-center justify-between mb-5">
                <div className="flex items-center gap-2">
                  <Shield className="h-4 w-4 text-muted-foreground" />
                  <span className="text-sm font-medium">凭据列表</span>
                  <Badge variant="secondary" className="text-xs">
                    {credentials.length} 个
                  </Badge>
                </div>
                <div className="flex gap-2">
                  <Button onClick={onImportCredentials} size="sm" variant="outline" className="rounded-lg">
                    <Upload className="h-4 w-4 mr-1.5" />
                    导入
                  </Button>
                  <Button 
                    onClick={onAddCredential} 
                    size="sm" 
                    className="rounded-lg bg-gradient-to-r from-indigo-500 to-purple-600 hover:from-indigo-600 hover:to-purple-700"
                  >
                    <Plus className="h-4 w-4 mr-1.5" />
                    添加
                  </Button>
                </div>
              </div>

              {credentials.length === 0 ? (
                <div className="text-center py-12 rounded-xl border-2 border-dashed border-muted-foreground/20">
                  <Key className="h-12 w-12 mx-auto mb-3 text-muted-foreground/40" />
                  <p className="text-muted-foreground mb-1">暂无凭据</p>
                  <p className="text-sm text-muted-foreground/70">点击"添加"或"导入"添加凭据</p>
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
            <div className="text-center py-8">
              <div className="grid grid-cols-3 gap-6 max-w-lg mx-auto">
                <div className="p-4 rounded-xl bg-background shadow-sm">
                  <div className="text-3xl font-bold">{pool.totalCredentials}</div>
                  <div className="text-xs text-muted-foreground mt-1">总凭据</div>
                </div>
                <div className="p-4 rounded-xl bg-background shadow-sm">
                  <div className="text-3xl font-bold text-green-600">{pool.availableCredentials}</div>
                  <div className="text-xs text-muted-foreground mt-1">可用</div>
                </div>
                <div className="p-4 rounded-xl bg-background shadow-sm">
                  <div className="text-3xl font-bold text-purple-600">{pool.sessionCacheSize}</div>
                  <div className="text-xs text-muted-foreground mt-1">会话缓存</div>
                </div>
              </div>
              <p className="text-sm text-muted-foreground mt-6">
                提示：将凭据分配到此池后，可在此查看详情
              </p>
            </div>
          )}
        </div>
      )}
    </Card>
  )
}
