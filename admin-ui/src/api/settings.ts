import api from './client'
import type {
  ConfigResponse,
  UpdateConfigRequest,
  ApiKeyItem,
  CreateApiKeyRequest,
  UpdateApiKeyRequest,
  SuccessResponse,
} from '@/types/api'

// ============ 配置管理 ============

// 获取配置
export async function getConfig(): Promise<ConfigResponse> {
  const { data } = await api.get<ConfigResponse>('/config')
  return data
}

// 更新配置
export async function updateConfig(config: UpdateConfigRequest): Promise<SuccessResponse> {
  const { data } = await api.put<SuccessResponse>('/config', config)
  return data
}

// ============ API Key 管理 ============

// 获取所有 API Keys
export async function getApiKeys(): Promise<ApiKeyItem[]> {
  const { data } = await api.get<ApiKeyItem[]>('/api-keys')
  return data
}

// 创建 API Key
export async function createApiKey(req: CreateApiKeyRequest): Promise<ApiKeyItem> {
  const { data } = await api.post<ApiKeyItem>('/api-keys', req)
  return data
}

// 更新 API Key
export async function updateApiKey(id: number, req: UpdateApiKeyRequest): Promise<ApiKeyItem> {
  const { data } = await api.put<ApiKeyItem>(`/api-keys/${id}`, req)
  return data
}

// 删除 API Key
export async function deleteApiKey(id: number): Promise<SuccessResponse> {
  const { data } = await api.delete<SuccessResponse>(`/api-keys/${id}`)
  return data
}
