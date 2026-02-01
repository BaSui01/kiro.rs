import api from './client'
import type {
  PoolsListResponse,
  PoolStatusItem,
  CreatePoolRequest,
  UpdatePoolRequest,
  SetPoolDisabledRequest,
  AssignCredentialToPoolRequest,
  SuccessResponse,
} from '@/types/api'

// 获取所有池
export async function fetchPools(): Promise<PoolsListResponse> {
  const { data } = await api.get<PoolsListResponse>('/pools')
  return data
}

// 获取单个池详情
export async function fetchPool(poolId: string): Promise<PoolStatusItem> {
  const { data } = await api.get<PoolStatusItem>(`/pools/${encodeURIComponent(poolId)}`)
  return data
}

// 创建新池
export async function createPool(request: CreatePoolRequest): Promise<SuccessResponse> {
  const { data } = await api.post<SuccessResponse>('/pools', request)
  return data
}

// 更新池配置
export async function updatePool(poolId: string, request: UpdatePoolRequest): Promise<SuccessResponse> {
  const { data } = await api.put<SuccessResponse>(`/pools/${encodeURIComponent(poolId)}`, request)
  return data
}

// 删除池
export async function deletePool(poolId: string): Promise<SuccessResponse> {
  const { data } = await api.delete<SuccessResponse>(`/pools/${encodeURIComponent(poolId)}`)
  return data
}

// 设置池禁用状态
export async function setPoolDisabled(poolId: string, request: SetPoolDisabledRequest): Promise<SuccessResponse> {
  const { data } = await api.post<SuccessResponse>(`/pools/${encodeURIComponent(poolId)}/disabled`, request)
  return data
}

// 将凭据分配到池
export async function assignCredentialToPool(
  credentialId: number,
  request: AssignCredentialToPoolRequest
): Promise<SuccessResponse> {
  const { data } = await api.post<SuccessResponse>(`/credentials/${credentialId}/pool`, request)
  return data
}
