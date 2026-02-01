import { useState, useCallback } from 'react'
import type { PoolStatusItem } from '@/types/api'

export interface DashboardDialogs {
  balance: boolean
  addCredential: boolean
  importCredentials: boolean
  poolDialog: boolean
}

export interface DashboardState {
  dialogs: DashboardDialogs
  selectedCredentialId: number | null
  editingPool: PoolStatusItem | null
  expandedPools: Set<string>
}

export interface DashboardStateActions {
  // Dialog actions
  openBalanceDialog: (credentialId: number) => void
  closeBalanceDialog: () => void
  openAddCredentialDialog: () => void
  closeAddCredentialDialog: () => void
  openImportCredentialsDialog: () => void
  closeImportCredentialsDialog: () => void
  openPoolDialog: (pool?: PoolStatusItem) => void
  closePoolDialog: () => void
  // Pool expansion actions
  togglePoolExpanded: (poolId: string) => void
  // Dark mode
  darkMode: boolean
  toggleDarkMode: () => void
}

export function useDashboardState(): DashboardState & DashboardStateActions {
  const [dialogs, setDialogs] = useState<DashboardDialogs>({
    balance: false,
    addCredential: false,
    importCredentials: false,
    poolDialog: false,
  })
  const [selectedCredentialId, setSelectedCredentialId] = useState<number | null>(null)
  const [editingPool, setEditingPool] = useState<PoolStatusItem | null>(null)
  const [expandedPools, setExpandedPools] = useState<Set<string>>(new Set(['default']))
  const [darkMode, setDarkMode] = useState(() => {
    if (typeof window !== 'undefined') {
      return document.documentElement.classList.contains('dark')
    }
    return false
  })

  // Dialog actions
  const openBalanceDialog = useCallback((credentialId: number) => {
    setSelectedCredentialId(credentialId)
    setDialogs((prev) => ({ ...prev, balance: true }))
  }, [])

  const closeBalanceDialog = useCallback(() => {
    setDialogs((prev) => ({ ...prev, balance: false }))
  }, [])

  const openAddCredentialDialog = useCallback(() => {
    setDialogs((prev) => ({ ...prev, addCredential: true }))
  }, [])

  const closeAddCredentialDialog = useCallback(() => {
    setDialogs((prev) => ({ ...prev, addCredential: false }))
  }, [])

  const openImportCredentialsDialog = useCallback(() => {
    setDialogs((prev) => ({ ...prev, importCredentials: true }))
  }, [])

  const closeImportCredentialsDialog = useCallback(() => {
    setDialogs((prev) => ({ ...prev, importCredentials: false }))
  }, [])

  const openPoolDialog = useCallback((pool?: PoolStatusItem) => {
    setEditingPool(pool || null)
    setDialogs((prev) => ({ ...prev, poolDialog: true }))
  }, [])

  const closePoolDialog = useCallback(() => {
    setDialogs((prev) => ({ ...prev, poolDialog: false }))
  }, [])

  // Pool expansion actions
  const togglePoolExpanded = useCallback((poolId: string) => {
    setExpandedPools((prev) => {
      const next = new Set(prev)
      if (next.has(poolId)) {
        next.delete(poolId)
      } else {
        next.add(poolId)
      }
      return next
    })
  }, [])

  // Dark mode
  const toggleDarkMode = useCallback(() => {
    setDarkMode((prev) => {
      const newValue = !prev
      document.documentElement.classList.toggle('dark')
      return newValue
    })
  }, [])

  return {
    // State
    dialogs,
    selectedCredentialId,
    editingPool,
    expandedPools,
    darkMode,
    // Actions
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
  }
}
