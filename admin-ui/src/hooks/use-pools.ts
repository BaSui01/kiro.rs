import { useState, useEffect, useCallback } from 'react'
import type { PoolStatusItem, CreatePoolRequest, UpdatePoolRequest } from '../types/api'
import {
  fetchPools,
  createPool as apiCreatePool,
  updatePool as apiUpdatePool,
  deletePool as apiDeletePool,
  setPoolDisabled as apiSetPoolDisabled,
  assignCredentialToPool as apiAssignCredentialToPool,
} from '../api/pools'

export function usePools() {
  const [pools, setPools] = useState<PoolStatusItem[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  const refresh = useCallback(async () => {
    try {
      setLoading(true)
      setError(null)
      const response = await fetchPools()
      setPools(response.pools)
    } catch (err) {
      setError(err instanceof Error ? err.message : '获取池列表失败')
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    refresh()
  }, [refresh])

  const createPool = useCallback(async (request: CreatePoolRequest) => {
    await apiCreatePool(request)
    await refresh()
  }, [refresh])

  const updatePool = useCallback(async (poolId: string, request: UpdatePoolRequest) => {
    await apiUpdatePool(poolId, request)
    await refresh()
  }, [refresh])

  const deletePool = useCallback(async (poolId: string) => {
    await apiDeletePool(poolId)
    await refresh()
  }, [refresh])

  const setPoolDisabled = useCallback(async (poolId: string, disabled: boolean) => {
    await apiSetPoolDisabled(poolId, { disabled })
    await refresh()
  }, [refresh])

  const assignCredentialToPool = useCallback(async (credentialId: number, poolId: string) => {
    await apiAssignCredentialToPool(credentialId, { poolId })
    await refresh()
  }, [refresh])

  return {
    pools,
    loading,
    error,
    refresh,
    createPool,
    updatePool,
    deletePool,
    setPoolDisabled,
    assignCredentialToPool,
  }
}
