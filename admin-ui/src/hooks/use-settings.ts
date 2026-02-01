import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import {
  getConfig,
  updateConfig,
  getApiKeys,
  createApiKey,
  updateApiKey,
  deleteApiKey,
} from '@/api/settings'
import type { UpdateConfigRequest, CreateApiKeyRequest, UpdateApiKeyRequest } from '@/types/api'

// ============ 配置管理 ============

// 获取配置
export function useConfig() {
  return useQuery({
    queryKey: ['config'],
    queryFn: getConfig,
    staleTime: 30 * 1000, // 30 秒
  })
}

// 更新配置
export function useUpdateConfig() {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: (config: UpdateConfigRequest) => updateConfig(config),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['config'] })
    },
  })
}

// ============ API Key 管理 ============

// 获取所有 API Keys
export function useApiKeys() {
  return useQuery({
    queryKey: ['api-keys'],
    queryFn: getApiKeys,
    staleTime: 30 * 1000,
  })
}

// 创建 API Key
export function useCreateApiKey() {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: (req: CreateApiKeyRequest) => createApiKey(req),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['api-keys'] })
    },
  })
}

// 更新 API Key
export function useUpdateApiKey() {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: ({ id, req }: { id: number; req: UpdateApiKeyRequest }) => updateApiKey(id, req),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['api-keys'] })
    },
  })
}

// 删除 API Key
export function useDeleteApiKey() {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: (id: number) => deleteApiKey(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['api-keys'] })
    },
  })
}
