import { Plus, Layers } from 'lucide-react'
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
  const sortedPools = [...pools].sort((a, b) => {
    if (a.id === 'default') return -1
    if (b.id === 'default') return 1
    return a.priority - b.priority
  })

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <div className="flex items-center justify-center w-10 h-10 rounded-xl bg-gradient-to-br from-indigo-500 to-purple-600 shadow-lg shadow-indigo-500/25">
            <Layers className="h-5 w-5 text-white" />
          </div>
          <div>
            <h2 className="text-xl font-bold">凭证池管理</h2>
            <p className="text-sm text-muted-foreground">管理和监控您的凭证池</p>
          </div>
        </div>
        <Button 
          onClick={onCreatePool} 
          className="bg-gradient-to-r from-indigo-500 to-purple-600 hover:from-indigo-600 hover:to-purple-700 shadow-lg shadow-indigo-500/25 transition-all"
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
            credentials={pool.id === 'default' ? defaultPoolCredentials : []}
            onViewBalance={onViewBalance}
            onAddCredential={onAddCredential}
            onImportCredentials={onImportCredentials}
          />
        ))}
      </div>
    </div>
  )
}
