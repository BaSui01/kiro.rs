// 凭据状态响应
export interface CredentialsStatusResponse {
  total: number
  available: number
  currentId: number
  credentials: CredentialStatusItem[]
  // 会话缓存统计
  sessionCacheSize: number
  roundRobinCounter: number
  // 调度模式
  schedulingMode: SchedulingMode
}

// 调度模式
export type SchedulingMode = 'round_robin' | 'priority_fill'

// 单个凭据状态
export interface CredentialStatusItem {
  id: number
  priority: number
  disabled: boolean
  failureCount: number
  isCurrent: boolean
  expiresAt: string | null
  authMethod: string | null
  hasProfileArn: boolean
  // ============ 调用统计字段 ============
  /** 成功调用次数（总计） */
  successCount: number
  /** 失败调用次数（总计） */
  totalFailureCount: number
  /** 总调用次数 */
  totalCalls: number
  /** 成功率（百分比，0-100） */
  successRate: number
  /** 最后调用时间（Unix 时间戳毫秒） */
  lastCallTime: number | null
  /** 平均响应时间（毫秒） */
  avgResponseTimeMs: number | null
  /** 今日成功调用次数 */
  todaySuccessCount: number
  /** 今日失败调用次数 */
  todayFailureCount: number
  /** 今日总调用次数 */
  todayTotalCalls: number
  // ============ Token 刷新统计字段 ============
  /** Token 刷新成功次数 */
  tokenRefreshCount: number
  /** Token 刷新失败次数 */
  tokenRefreshFailureCount: number
  /** 最后 Token 刷新时间（Unix 时间戳毫秒） */
  lastTokenRefreshTime: number | null
}

// 余额响应
export interface BalanceResponse {
  id: number
  subscriptionTitle: string | null
  currentUsage: number
  usageLimit: number
  remaining: number
  usagePercentage: number
  nextResetAt: number | null
}

// 成功响应
export interface SuccessResponse {
  success: boolean
  message: string
}

// CSRF Token 响应
export interface CsrfTokenResponse {
  token: string
}

// 错误响应
export interface AdminErrorResponse {
  error: {
    type: string
    message: string
  }
}

// 请求类型
export interface SetDisabledRequest {
  disabled: boolean
}

export interface SetPriorityRequest {
  priority: number
}

// 添加凭据请求
export interface AddCredentialRequest {
  refreshToken: string
  authMethod?: 'social' | 'idc'
  clientId?: string
  clientSecret?: string
  priority?: number
  region?: string
  machineId?: string
  poolId?: string
  // 凭据级代理配置
  proxyUrl?: string
  proxyUsername?: string
  proxyPassword?: string
}

// 添加凭据响应
export interface AddCredentialResponse {
  success: boolean
  message: string
  credentialId: number
}

// ============ 批量导入凭据 ============

// IdC 格式的凭据（从 Kiro Account Manager 导出）
export interface IdcCredentialItem {
  email?: string
  label?: string
  accessToken?: string
  refreshToken?: string
  expiresAt?: string
  provider?: string
  clientId?: string
  clientSecret?: string
  region?: string
}

// 批量导入凭据请求
export interface ImportCredentialsRequest {
  credentials: IdcCredentialItem[]
  poolId?: string // 导入到指定池（可选，默认为 default）
}

// 批量导入凭据响应
export interface ImportCredentialsResponse {
  success: boolean
  message: string
  importedCount: number
  skippedCount: number
  credentialIds: number[]
  skippedItems: string[]
}

// ============ 配置管理 ============

// 配置响应
export interface ConfigResponse {
  host: string
  port: number
  region: string
  kiroVersion: string
  tlsBackend: 'rustls' | 'native-tls'
  sessionCacheMaxCapacity: number
  sessionCacheTtlSecs: number
  proxyUrl: string | null
  proxyUsername: string | null
  proxyPassword: string | null
  hasApiKey: boolean
  hasAdminApiKey: boolean
}

// 更新配置请求
export interface UpdateConfigRequest {
  host?: string
  port?: number
  region?: string
  sessionCacheMaxCapacity?: number
  sessionCacheTtlSecs?: number
  proxyUrl?: string
  proxyUsername?: string
  proxyPassword?: string
  apiKey?: string
}

// ============ API Key 管理 ============

// API Key 条目
export interface ApiKeyItem {
  id: number
  name: string
  key: string // 脱敏显示
  description: string | null
  createdAt: string
  enabled: boolean
  poolId: string | null // 绑定的池 ID
}

// 创建 API Key 请求
export interface CreateApiKeyRequest {
  name: string
  description?: string
  key?: string // 可选，不提供则自动生成
  poolId?: string // 绑定的池 ID
}

// 更新 API Key 请求
export interface UpdateApiKeyRequest {
  name?: string
  description?: string
  enabled?: boolean
  poolId?: string | null // 绑定的池 ID，设为 null 可解绑
}

// ============ 池管理 ============

// 池状态
export interface PoolStatusItem {
  id: string
  name: string
  description: string | null
  enabled: boolean
  schedulingMode: SchedulingMode
  hasProxy: boolean
  priority: number
  totalCredentials: number
  availableCredentials: number
  currentId: number
  sessionCacheSize: number
  roundRobinCounter: number
}

// 池列表响应
export interface PoolsListResponse {
  pools: PoolStatusItem[]
}

// 创建池请求
export interface CreatePoolRequest {
  id: string
  name: string
  description?: string
  schedulingMode?: SchedulingMode
  proxyUrl?: string
  proxyUsername?: string
  proxyPassword?: string
  priority?: number
}

// 更新池请求
export interface UpdatePoolRequest {
  name?: string
  description?: string
  enabled?: boolean
  schedulingMode?: SchedulingMode
  proxyUrl?: string
  proxyUsername?: string
  proxyPassword?: string
  priority?: number
}

// 设置池禁用状态请求
export interface SetPoolDisabledRequest {
  disabled: boolean
}

// 分配凭据到池请求
export interface AssignCredentialToPoolRequest {
  poolId: string
}

// 池凭证列表响应
export interface PoolCredentialsResponse {
  poolId: string
  total: number
  available: number
  currentId: number
  credentials: CredentialStatusItem[]
  schedulingMode: SchedulingMode
}
