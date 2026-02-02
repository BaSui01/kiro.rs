import { Plus, Layers } from "lucide-react";
import { Button } from "@/components/ui/button";
import { PoolItem } from "./pool-item";
import type {
  PoolStatusItem,
  CredentialStatusItem,
  PoolCredentialsResponse,
} from "@/types/api";

export interface PoolListProps {
  pools: PoolStatusItem[];
  expandedPools: Set<string>;
  onTogglePoolExpanded: (poolId: string) => void;
  onCreatePool: () => void;
  onEditPool: (pool: PoolStatusItem) => void;
  onDeletePool: (poolId: string) => void;
  onTogglePoolEnabled: (poolId: string, enabled: boolean) => void;
  defaultPoolCredentials: CredentialStatusItem[];
  onViewBalance: (id: number) => void;
  onAddCredential: () => void;
  onImportCredentials: () => void;
  // 新增：获取池凭证列表的方法
  fetchPoolCredentials?: (poolId: string) => Promise<PoolCredentialsResponse>;
  // 新增：转移凭证的方法
  onTransferCredential?: (
    credentialId: number,
    targetPoolId: string
  ) => Promise<void>;
}

export function PoolList({
  pools,
  expandedPools,
  onTogglePoolExpanded,
  onCreatePool,
  onEditPool,
  onDeletePool,
  onTogglePoolEnabled,
  defaultPoolCredentials,
  onViewBalance,
  onAddCredential,
  onImportCredentials,
  fetchPoolCredentials,
  onTransferCredential,
}: PoolListProps) {
  const sortedPools = [...pools].sort((a, b) => {
    if (a.id === "default") return -1;
    if (b.id === "default") return 1;
    return a.priority - b.priority;
  });

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <div className="flex items-center justify-center w-10 h-10 rounded-xl bg-primary shadow-lg shadow-primary/25">
            <Layers className="h-5 w-5 text-primary-foreground" />
          </div>
          <div>
            <h2 className="text-xl font-bold">凭证池管理</h2>
            <p className="text-sm text-muted-foreground">
              管理和监控您的凭证池
            </p>
          </div>
        </div>
        <Button
          onClick={onCreatePool}
          className="bg-primary hover:bg-primary/90 shadow-lg shadow-primary/25 transition-all"
        >
          <Plus className="h-4 w-4 mr-2" />
          创建池
        </Button>
      </div>

      <div className="space-y-4">
        {sortedPools.map((pool) => (
          <PoolItem
            key={pool.id}
            pool={pool}
            expanded={expandedPools.has(pool.id)}
            onToggleExpand={() => onTogglePoolExpanded(pool.id)}
            onEdit={() => onEditPool(pool)}
            onDelete={() => onDeletePool(pool.id)}
            onToggleEnabled={(enabled) => onTogglePoolEnabled(pool.id, enabled)}
            credentials={pool.id === "default" ? defaultPoolCredentials : []}
            onViewBalance={onViewBalance}
            onAddCredential={onAddCredential}
            onImportCredentials={onImportCredentials}
            fetchPoolCredentials={fetchPoolCredentials}
            onTransferCredential={onTransferCredential}
            allPools={pools}
          />
        ))}
      </div>
    </div>
  );
}
