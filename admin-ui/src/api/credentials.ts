import api, { getCsrfToken, initCsrfToken } from './client'
import type {
  CredentialsStatusResponse,
  BalanceResponse,
  SuccessResponse,
  SetDisabledRequest,
  SetPriorityRequest,
  AddCredentialRequest,
  AddCredentialResponse,
  ImportCredentialsRequest,
  ImportCredentialsResponse,
  SchedulingMode,
  CsrfTokenResponse,
} from '@/types/api'

// 导出 CSRF Token 相关函数
export { getCsrfToken, initCsrfToken }
export type { CsrfTokenResponse }

// 获取所有凭据状态
export async function getCredentials(): Promise<CredentialsStatusResponse> {
  const { data } = await api.get<CredentialsStatusResponse>('/credentials')
  return data
}

// 设置凭据禁用状态
export async function setCredentialDisabled(
  id: number,
  disabled: boolean
): Promise<SuccessResponse> {
  const { data } = await api.post<SuccessResponse>(
    `/credentials/${id}/disabled`,
    { disabled } as SetDisabledRequest
  )
  return data
}

// 设置凭据优先级
export async function setCredentialPriority(
  id: number,
  priority: number
): Promise<SuccessResponse> {
  const { data } = await api.post<SuccessResponse>(
    `/credentials/${id}/priority`,
    { priority } as SetPriorityRequest
  )
  return data
}

// 重置失败计数
export async function resetCredentialFailure(
  id: number
): Promise<SuccessResponse> {
  const { data } = await api.post<SuccessResponse>(`/credentials/${id}/reset`)
  return data
}

// 获取凭据余额
export async function getCredentialBalance(id: number): Promise<BalanceResponse> {
  const { data } = await api.get<BalanceResponse>(`/credentials/${id}/balance`)
  return data
}

// 添加新凭据
export async function addCredential(
  req: AddCredentialRequest
): Promise<AddCredentialResponse> {
  const { data } = await api.post<AddCredentialResponse>('/credentials', req)
  return data
}

// 删除凭据
export async function deleteCredential(id: number): Promise<SuccessResponse> {
  const { data } = await api.delete<SuccessResponse>(`/credentials/${id}`)
  return data
}

// 批量导入凭据（支持 IdC 格式）
export async function importCredentials(
  req: ImportCredentialsRequest
): Promise<ImportCredentialsResponse> {
  const { data } = await api.post<ImportCredentialsResponse>('/credentials/import', req)
  return data
}

// 设置调度模式
export async function setSchedulingMode(mode: SchedulingMode): Promise<SuccessResponse> {
  const { data } = await api.post<SuccessResponse>('/scheduling-mode', { mode })
  return data
}
