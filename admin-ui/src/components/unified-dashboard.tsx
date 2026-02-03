import { useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { storage } from "@/lib/storage";
import { BalanceDialog } from "@/components/balance-dialog";
import { AddCredentialDialog } from "@/components/add-credential-dialog";
import { ImportCredentialsDialog } from "@/components/import-credentials-dialog";
import { PoolDialog } from "@/components/pool-dialog";
import { DashboardHeader } from "@/components/dashboard/dashboard-header";
import { DashboardStats } from "@/components/dashboard/dashboard-stats";
import { PoolList } from "@/components/dashboard/pool-list";
import { useCredentials } from "@/hooks/use-credentials";
import { usePools } from "@/hooks/use-pools";
import { useDashboardState } from "@/hooks/use-dashboard-state";
import { FadeIn, SlideIn } from "@/components/ui/motion";
import type {
  PoolStatusItem,
  CreatePoolRequest,
  UpdatePoolRequest,
} from "@/types/api";

interface UnifiedDashboardProps {
  onLogout: () => void;
  onSettings: () => void;
}

export function UnifiedDashboard({
  onLogout,
  onSettings,
}: UnifiedDashboardProps) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const {
    data: credentialsData,
    isLoading: credentialsLoading,
    refetch: refetchCredentials,
  } = useCredentials();
  const {
    pools,
    loading: poolsLoading,
    refresh: refetchPools,
    createPool,
    updatePool,
    deletePool,
    setPoolDisabled,
    assignCredentialToPool,
    fetchPoolCredentials,
  } = usePools();

  const {
    dialogs,
    selectedCredentialId,
    editingPool,
    expandedPools,
    darkMode,
    importTargetPoolId, // 新增：导入目标池ID 🎯
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
  } = useDashboardState();

  const handleRefresh = () => {
    refetchCredentials();
    refetchPools();
    toast.success(t('common.refreshed'));
  };

  const handleLogout = () => {
    storage.removeApiKey();
    queryClient.clear();
    onLogout();
  };

  const handleCreatePool = () => {
    openPoolDialog();
  };

  const handleEditPool = (pool: PoolStatusItem) => {
    openPoolDialog(pool);
  };

  const handleDeletePool = async (poolId: string) => {
    if (!confirm(t('dashboard.deletePoolConfirm', { poolId }))) {
      return;
    }
    try {
      await deletePool(poolId);
      toast.success(t('dashboard.poolDeleted', { poolId }));
    } catch (err) {
      toast.error(err instanceof Error ? err.message : t('dashboard.deletePoolFailed'));
    }
  };

  const handleTogglePoolEnabled = async (poolId: string, enabled: boolean) => {
    try {
      await setPoolDisabled(poolId, !enabled);
      toast.success(t(enabled ? 'dashboard.poolEnabled' : 'dashboard.poolDisabled'));
    } catch (err) {
      toast.error(err instanceof Error ? err.message : t('dashboard.operationFailed'));
    }
  };

  const handlePoolSubmit = async (
    data: CreatePoolRequest | UpdatePoolRequest
  ) => {
    try {
      if (editingPool) {
        await updatePool(editingPool.id, data as UpdatePoolRequest);
        toast.success(t('dashboard.poolUpdated', { poolId: editingPool.id }));
      } else {
        await createPool(data as CreatePoolRequest);
        toast.success(t('dashboard.poolCreated', { poolId: (data as CreatePoolRequest).id }));
      }
    } catch (err) {
      toast.error(err instanceof Error ? err.message : t('dashboard.operationFailed'));
      throw err;
    }
  };

  // 处理凭证转移
  const handleTransferCredential = async (
    credentialId: number,
    targetPoolId: string
  ) => {
    await assignCredentialToPool(credentialId, targetPoolId);
    // 刷新凭证列表
    refetchCredentials();
  };

  const isLoading = credentialsLoading || poolsLoading;

  if (isLoading) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-gradient-to-br from-background via-background to-muted/30">
        <div className="text-center">
          <div className="relative">
            <div className="animate-spin rounded-full h-16 w-16 border-4 border-muted border-t-primary mx-auto mb-4"></div>
            <div className="absolute inset-0 rounded-full h-16 w-16 border-4 border-transparent border-t-primary/30 animate-ping mx-auto"></div>
          </div>
          <p className="text-muted-foreground font-medium">{t('common.loading')}</p>
        </div>
      </div>
    );
  }

  // 计算统计数据
  const stats = {
    totalPools: pools.length,
    enabledPools: pools.filter((p) => p.enabled).length,
    totalCredentials: pools.reduce((sum, p) => sum + p.totalCredentials, 0),
    availableCredentials: pools.reduce(
      (sum, p) => sum + p.availableCredentials,
      0
    ),
    sessionCacheSize: pools.reduce((sum, p) => sum + p.sessionCacheSize, 0),
    roundRobinCounter: pools.reduce((sum, p) => sum + p.roundRobinCounter, 0),
  };

  return (
    <div className="min-h-screen bg-gradient-to-br from-background via-background to-muted/20">
      <FadeIn>
        <DashboardHeader
          darkMode={darkMode}
          onToggleDarkMode={toggleDarkMode}
          onRefresh={handleRefresh}
          onSettings={onSettings}
          onLogout={handleLogout}
        />
      </FadeIn>

      <main className="container px-4 md:px-8 py-8">
        <SlideIn direction="up" delay={0.1}>
          <DashboardStats stats={stats} />
        </SlideIn>
        <SlideIn direction="up" delay={0.2}>
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
            fetchPoolCredentials={fetchPoolCredentials}
            onTransferCredential={handleTransferCredential}
          />
        </SlideIn>
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
        defaultPoolId={importTargetPoolId}
      />

      {/* 池对话框 */}
      <PoolDialog
        open={dialogs.poolDialog}
        onOpenChange={(open) => !open && closePoolDialog()}
        pool={editingPool}
        onSubmit={handlePoolSubmit}
      />
    </div>
  );
}
