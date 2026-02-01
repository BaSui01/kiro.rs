import { useState } from 'react'
import { Button } from './ui/button'
import { Card, CardContent } from './ui/card'
import { PoolCard } from './pool-card'
import { PoolDialog } from './pool-dialog'
import { usePools } from '../hooks/use-pools'
import type { PoolStatusItem, CreatePoolRequest, UpdatePoolRequest } from '../types/api'
import { Plus, RefreshCw, Layers, Database, CheckCircle2, Key } from 'lucide-react'
import { toast } from 'sonner'

export function PoolManagement() {
  const { pools, loading, error, refresh, createPool, updatePool, deletePool, setPoolDisabled } = usePools()
  const [dialogOpen, setDialogOpen] = useState(false)
  const [editingPool, setEditingPool] = useState<PoolStatusItem | null>(null)

  const handleCreatePool = () => {
    setEditingPool(null)
    setDialogOpen(true)
  }

  const handleEditPool = (pool: PoolStatusItem) => {
    setEditingPool(pool)
    setDialogOpen(true)
  }

  const handleDeletePool = async (poolId: string) => {
    if (!confirm(`确定要删除池 "${poolId}" 吗？此操作不可撤销。`)) {
      return
    }
    try {
      await deletePool(poolId)
      toast.success(`池 ${poolId} 已删除`)
    } catch (err) {
      toast.error(err instanceof Error ? err.message : '删除池失败')
    }
  }

  const handleToggleEnabled = async (poolId: string, enabled: boolean) => {
    try {
      await setPoolDisabled(poolId, !enabled)
      toast.success(`池 ${poolId} 已${enabled ? '启用' : '禁用'}`)
    } catch (err) {
      toast.error(err instanceof Error ? err.message : '操作失败')
    }
  }

  const handleSubmit = async (data: CreatePoolRequest | UpdatePoolRequest) => {
    try {
      if (editingPool) {
        await updatePool(editingPool.id, data as UpdatePoolRequest)
        toast.success(`池 ${editingPool.id} 已更新`)
      } else {
        await createPool(data as CreatePoolRequest)
        toast.success(`池 ${(data as CreatePoolRequest).id} 已创建`)
      }
    } catch (err) {
      toast.error(err instanceof Error ? err.message : '操作失败')
      throw err
    }
  }

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <RefreshCw className="h-8 w-8 animate-spin text-muted-foreground" />
      </div>
    )
  }

  if (error) {
    return (
      <div className="flex flex-col items-center justify-center h-64 gap-4">
        <p className="text-destructive">{error}</p>
        <Button onClick={refresh} variant="outline">
          <RefreshCw className="h-4 w-4 mr-2" />
          重试
        </Button>
      </div>
    )
  }

  const totalCredentials = pools.reduce((sum, p) => sum + p.totalCredentials, 0)
  const enabledPools = pools.filter((p) => p.enabled).length

  return (
    <div className="space-y-6">
      {/* 头部 */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <div className="p-2 bg-gradient-to-br from-cyan-500 to-blue-600 rounded-lg">
            <Layers className="h-6 w-6 text-white" />
          </div>
          <div>
            <h2 className="text-2xl font-bold">凭证池管理</h2>
            <p className="text-sm text-muted-foreground">管理和配置凭证池</p>
          </div>
        </div>
        <div className="flex gap-2">
          <Button onClick={refresh} variant="outline" size="sm">
            <RefreshCw className="h-4 w-4 mr-1" />
            刷新
          </Button>
          <Button onClick={handleCreatePool} size="sm" className="bg-gradient-to-r from-cyan-500 to-blue-600 hover:from-cyan-600 hover:to-blue-700">
            <Plus className="h-4 w-4 mr-1" />
            创建池
          </Button>
        </div>
      </div>

      {/* 统计信息 */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        <Card className="border-l-4 border-l-cyan-500">
          <CardContent className="pt-4">
            <div className="flex items-center gap-3">
              <div className="p-2 bg-cyan-100 dark:bg-cyan-900/30 rounded-lg">
                <Database className="h-5 w-5 text-cyan-600" />
              </div>
              <div>
                <p className="text-sm text-muted-foreground">总池数</p>
                <p className="text-2xl font-bold">{pools.length}</p>
              </div>
            </div>
          </CardContent>
        </Card>
        <Card className="border-l-4 border-l-green-500">
          <CardContent className="pt-4">
            <div className="flex items-center gap-3">
              <div className="p-2 bg-green-100 dark:bg-green-900/30 rounded-lg">
                <CheckCircle2 className="h-5 w-5 text-green-600" />
              </div>
              <div>
                <p className="text-sm text-muted-foreground">启用池数</p>
                <p className="text-2xl font-bold">{enabledPools}</p>
              </div>
            </div>
          </CardContent>
        </Card>
        <Card className="border-l-4 border-l-orange-500">
          <CardContent className="pt-4">
            <div className="flex items-center gap-3">
              <div className="p-2 bg-orange-100 dark:bg-orange-900/30 rounded-lg">
                <Key className="h-5 w-5 text-orange-600" />
              </div>
              <div>
                <p className="text-sm text-muted-foreground">总凭据数</p>
                <p className="text-2xl font-bold">{totalCredentials}</p>
              </div>
            </div>
          </CardContent>
        </Card>
      </div>

      {/* 池列表 */}
      {pools.length === 0 ? (
        <Card className="border-dashed">
          <CardContent className="py-12">
            <div className="text-center">
              <div className="mx-auto w-16 h-16 bg-muted rounded-full flex items-center justify-center mb-4">
                <Layers className="h-8 w-8 text-muted-foreground" />
              </div>
              <h3 className="text-lg font-medium mb-2">暂无凭证池</h3>
              <p className="text-sm text-muted-foreground mb-4">点击"创建池"按钮添加第一个池</p>
              <Button onClick={handleCreatePool} className="bg-gradient-to-r from-cyan-500 to-blue-600">
                <Plus className="h-4 w-4 mr-1" />
                创建池
              </Button>
            </div>
          </CardContent>
        </Card>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          {pools
            .sort((a, b) => {
              // 默认池排在最前面
              if (a.id === 'default') return -1
              if (b.id === 'default') return 1
              // 然后按优先级排序
              return a.priority - b.priority
            })
            .map((pool) => (
              <PoolCard
                key={pool.id}
                pool={pool}
                onToggleEnabled={handleToggleEnabled}
                onEdit={handleEditPool}
                onDelete={handleDeletePool}
              />
            ))}
        </div>
      )}

      {/* 池对话框 */}
      <PoolDialog
        open={dialogOpen}
        onOpenChange={setDialogOpen}
        pool={editingPool}
        onSubmit={handleSubmit}
      />
    </div>
  )
}
