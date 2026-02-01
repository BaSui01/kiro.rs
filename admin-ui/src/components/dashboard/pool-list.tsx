import { Plus } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { PoolItem } from './pool-item'
import type { PoolStatusItem, CredentialStatusItem } from '@/types/api'

export interface PoolListProps {
  pools: PoolStatusItem[]
  expandedPools: Set<string>
  onTogglePoolExpanded: (poolId: string) => void
  onCreatePool: () => void
  onEditPool: (pool: PoolStatusItem) => void
  onDeletePool: (poolId: string) => void
  onTogglePoolEnabled: (poolId: string, enabled: boolean) => void
  // Credentials for default pool
  defaultPoolCredentials: CredentialStatusItem[]
  onViewBalance: (id: number) => void
  onAddCredential: () => void
  onImportCredentials: () => void
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
}: PoolListProps) {
  // 排序池：默认池在前，然后按优先级排序
  const sortedPools = [...pools].sort((a, b) => {
    if (a.id === 'default') return -1
    if (b.id === 'default') return 1
    return a.priority - b.priority
  })

  return (
    <>
      {/* 池管理标题 */}
      <div className="flex items-center justify-between mb-4">
        <h2 className="text-xl font-semibold">凭证池管理</h2>
        <Button onClick={onCreatePool} size="sm">
          <Plus className="h-4 w-4 mr-2" />
          创建池
        </Button>
      </div>

      {/* 池列表 */}
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
            credentials={pool.id === 'default' ? defaultPoolCredentials : []}
            onViewBalance={onViewBalance}
            onAddCredential={onAddCredential}
            onImportCredentials={onImportCredentials}
          />
        ))}
      </div>
    </>
  )
}
