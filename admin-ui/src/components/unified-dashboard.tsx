import { useQueryClient } from '@tanstack/react-query'
import { toast } from 'sonner'
import { storage } from '@/lib/storage'
import { BalanceDialog } from '@/components/balance-dialog'
import { AddCredentialDialog } from '@/components/add-credential-dialog'
import { ImportCredentialsDialog } from '@/components/import-credentials-dialog'
import { PoolDialog } from '@/components/pool-dialog'
import { DashboardHeader } from '@/components/dashboard/dashboard-header'
import { DashboardStats } from '@/components/dashboard/dashboard-stats'
import { PoolList } from '@/components/dashboard/pool-list'
import { useCredentials } from '@/hooks/use-credentials'
import { usePools } from '@/hooks/use-pools'
import { useDashboardState } from '@/hooks/use-dashboard-state'
import type { PoolStatusItem, CreatePoolRequest, UpdatePoolRequest } from '@/types/api'

interface UnifiedDashboardProps {
  onLogout: () => void
  onSettings: () => void
}

export function UnifiedDashboard({ onLogout, onSettings }: UnifiedDashboardProps) {
  const queryClient = useQueryClient()
  const { data: credentialsData, isLoading: credentialsLoading, refetch: refetchCredentials } = useCredentials()
  const { pools, loading: poolsLoading, refresh: refetchPools, createPool, updatePool, deletePool, setPoolDisabled } = usePools()

  const {
    dialogs,
    selectedCredentialId,
    editingPool,
    expandedPools,
    darkMode,
    openBalanceDialog,
    closeBalanceDialog,
    openAddCredentialDialog,
    closeAddCredentialDialog,
    openImportCredentialsDialog,
    closeImportCredentialsDialog,
    openPoolDialog,
    closePoolDialog,
    togglePoolExpanded,
    toggleDarkMode,
  } = useDashboardState()

  const handleRefresh = () => {
    refetchCredentials()
    refetchPools()
    toast.success('已刷新')
  }

  const handleLogout = () => {
    storage.removeApiKey()
    queryClient.clear()
    onLogout()
  }

  const handleCreatePool = () => {
    openPoolDialog()
  }

  const handleEditPool = (pool: PoolStatusItem) => {
    openPoolDialog(pool)
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

  const handleTogglePoolEnabled = async (poolId: string, enabled: boolean) => {
    try {
      await setPoolDisabled(poolId, !enabled)
      toast.success(`池 ${poolId} 已${enabled ? '启用' : '禁用'}`)
    } catch (err) {
      toast.error(err instanceof Error ? err.message : '操作失败')
    }
  }

  const handlePoolSubmit = async (data: CreatePoolRequest | UpdatePoolRequest) => {
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

  const isLoading = credentialsLoading || poolsLoading

  if (isLoading) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-gradient-to-br from-background via-background to-muted/30">
        <div className="text-center">
          <div className="relative">
            <div className="animate-spin rounded-full h-16 w-16 border-4 border-muted border-t-violet-500 mx-auto mb-4"></div>
            <div className="absolute inset-0 rounded-full h-16 w-16 border-4 border-transparent border-t-purple-500/30 animate-ping mx-auto"></div>
          </div>
          <p className="text-muted-foreground font-medium">加载中...</p>
        </div>
      </div>
    )
  }

  // 计算统计数据
  const stats = {
    totalPools: pools.length,
    enabledPools: pools.filter((p) => p.enabled).length,
    totalCredentials: pools.reduce((sum, p) => sum + p.totalCredentials, 0),
    availableCredentials: pools.reduce((sum, p) => sum + p.availableCredentials, 0),
    sessionCacheSize: pools.reduce((sum, p) => sum + p.sessionCacheSize, 0),
    roundRobinCounter: pools.reduce((sum, p) => sum + p.roundRobinCounter, 0),
  }

  return (
    <div className="min-h-screen bg-gradient-to-br from-background via-background to-muted/20">
      <DashboardHeader
        darkMode={darkMode}
        onToggleDarkMode={toggleDarkMode}
        onRefresh={handleRefresh}
        onSettings={onSettings}
        onLogout={handleLogout}
      />

      <main className="container px-4 md:px-8 py-8">
        <DashboardStats stats={stats} />
        <PoolList
          pools={pools}
          expandedPools={expandedPools}
          onTogglePoolExpanded={togglePoolExpanded}
          onCreatePool={handleCreatePool}
          onEditPool={handleEditPool}
          onDeletePool={handleDeletePool}
          onTogglePoolEnabled={handleTogglePoolEnabled}
          defaultPoolCredentials={credentialsData?.credentials || []}
          onViewBalance={openBalanceDialog}
          onAddCredential={openAddCredentialDialog}
          onImportCredentials={openImportCredentialsDialog}
        />
      </main>

      {/* 余额对话框 */}
      <BalanceDialog
        credentialId={selectedCredentialId}
        open={dialogs.balance}
        onOpenChange={(open) => !open && closeBalanceDialog()}
      />

      {/* 添加凭据对话框 */}
      <AddCredentialDialog
        open={dialogs.addCredential}
        onOpenChange={(open) => !open && closeAddCredentialDialog()}
      />

      {/* 导入凭据对话框 */}
      <ImportCredentialsDialog
        open={dialogs.importCredentials}
        onOpenChange={(open) => !open && closeImportCredentialsDialog()}
      />

      {/* 池对话框 */}
      <PoolDialog
        open={dialogs.poolDialog}
        onOpenChange={(open) => !open && closePoolDialog()}
        pool={editingPool}
        onSubmit={handlePoolSubmit}
      />
    </div>
  )
}
