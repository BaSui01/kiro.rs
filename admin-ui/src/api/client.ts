/**
 * 共享的 API 客户端模块
 *
 * 提供带有 API Key 和 CSRF Token 自动处理的 axios 实例
 *
 * 安全特性：
 * - 自动添加 API Key 到请求头
 * - 自动处理 CSRF Token（一次性使用）
 * - 使用 Promise 缓存防止并发获取 Token
 * - 统一的错误处理和重试逻辑
 */

import axios, { type AxiosInstance, type AxiosError } from 'axios'
import { toast } from 'sonner'
import { storage } from '@/lib/storage'
import type { CsrfTokenResponse, AdminErrorResponse } from '@/types/api'

// 创建 axios 实例
const api: AxiosInstance = axios.create({
  baseURL: '/api/admin',
  timeout: 30000, // 30 秒超时
  headers: {
    'Content-Type': 'application/json',
  },
})

// ============ CSRF Token 管理（防止竞态条件） ============

// CSRF Token 获取的 Promise 缓存（防止并发请求时多次获取）
let csrfTokenPromise: Promise<string> | null = null

/**
 * 获取 CSRF Token（内部使用，带竞态条件保护）
 *
 * 使用 Promise 缓存确保并发请求只会获取一次 Token
 */
async function fetchCsrfTokenSafe(): Promise<string> {
  // 如果已经有正在进行的请求，等待它完成
  if (csrfTokenPromise) {
    return csrfTokenPromise
  }

  // 创建新的请求
  csrfTokenPromise = (async () => {
    try {
      const apiKey = storage.getApiKey()
      const { data } = await axios.get<CsrfTokenResponse>('/api/admin/csrf-token', {
        headers: {
          'x-api-key': apiKey || '',
        },
        timeout: 10000, // 10 秒超时
      })
      storage.setCsrfToken(data.token)
      return data.token
    } finally {
      // 请求完成后清除缓存，允许下次获取
      csrfTokenPromise = null
    }
  })()

  return csrfTokenPromise
}

// ============ 请求拦截器 ============

api.interceptors.request.use(async (config) => {
  // 添加 API Key
  const apiKey = storage.getApiKey()
  if (apiKey) {
    config.headers['x-api-key'] = apiKey
  }

  // 对 POST/PUT/DELETE 请求添加 CSRF Token
  const method = config.method?.toUpperCase()
  if (method === 'POST' || method === 'PUT' || method === 'DELETE') {
    let csrfToken = storage.getCsrfToken()

    // 如果没有 CSRF Token，先获取一个（带竞态保护）
    if (!csrfToken && apiKey) {
      try {
        csrfToken = await fetchCsrfTokenSafe()
      } catch (error) {
        console.error('Failed to fetch CSRF token:', error)
      }
    }

    if (csrfToken) {
      config.headers['x-csrf-token'] = csrfToken
      // CSRF Token 是一次性的，使用后清除
      storage.removeCsrfToken()
    }
  }

  return config
})

// ============ 响应拦截器（统一错误处理） ============

api.interceptors.response.use(
  (response) => {
    // 成功响应后，预获取下一个 CSRF Token（非阻塞）
    const method = response.config.method?.toUpperCase()
    if (method === 'POST' || method === 'PUT' || method === 'DELETE') {
      // 异步预获取，不阻塞当前响应
      fetchCsrfTokenSafe().catch((error) => {
        // 记录预获取失败，便于调试
        console.debug('CSRF Token 预获取失败（将在下次请求时重试）:', error)
      })
    }
    return response
  },
  async (error: AxiosError<AdminErrorResponse>) => {
    const status = error.response?.status
    const errorData = error.response?.data

    // 处理 CSRF Token 过期（403 + csrf_error）
    if (status === 403 && errorData?.error?.type === 'csrf_error') {
      console.warn('CSRF Token 过期，正在重新获取...')
      // 清除旧 Token 并获取新的
      storage.removeCsrfToken()
      try {
        await fetchCsrfTokenSafe()
        // 检查原请求配置是否存在
        if (!error.config) {
          console.error('无法重试请求：原请求配置丢失')
          toast.error('CSRF 验证失败，请刷新页面重试')
          return Promise.reject(error)
        }
        // 重试原请求
        return api.request(error.config)
      } catch (retryError) {
        toast.error('CSRF 验证失败，请刷新页面重试')
        return Promise.reject(retryError)
      }
    }

    // 处理认证错误（401）
    if (status === 401) {
      toast.error('认证失败，请重新登录')
      storage.removeApiKey()
      // 可以在这里触发重定向到登录页
      window.location.reload()
      return Promise.reject(error)
    }

    // 处理权限错误（403，非 CSRF）
    if (status === 403) {
      toast.error('您没有权限执行此操作')
      return Promise.reject(error)
    }

    // 处理服务不可用（503）
    if (status === 503) {
      toast.error('服务暂时不可用，请稍后重试')
      return Promise.reject(error)
    }

    // 处理网络错误
    if (error.code === 'ECONNABORTED') {
      toast.error('请求超时，请检查网络连接')
      return Promise.reject(error)
    }

    if (!error.response) {
      toast.error('网络连接失败，请检查网络')
      return Promise.reject(error)
    }

    // 其他错误，显示服务器返回的错误信息
    if (errorData?.error?.message) {
      // 不自动显示 toast，让调用方决定如何处理
      // toast.error(errorData.error.message)
    }

    return Promise.reject(error)
  }
)

// ============ 公开 API ============

/**
 * 获取 CSRF Token（公开 API）
 */
export async function getCsrfToken(): Promise<CsrfTokenResponse> {
  const { data } = await api.get<CsrfTokenResponse>('/csrf-token')
  storage.setCsrfToken(data.token)
  return data
}

/**
 * 初始化 CSRF Token（登录成功后调用）
 */
export async function initCsrfToken(): Promise<void> {
  try {
    await fetchCsrfTokenSafe()
  } catch (error) {
    console.error('Failed to initialize CSRF token:', error)
  }
}

/**
 * 提取错误信息
 *
 * 从 AxiosError 中提取用户友好的错误信息
 */
export function extractErrorMessage(error: unknown): string {
  if (axios.isAxiosError(error)) {
    const axiosError = error as AxiosError<AdminErrorResponse>
    if (axiosError.response?.data?.error?.message) {
      return axiosError.response.data.error.message
    }
    if (axiosError.message) {
      return axiosError.message
    }
  }
  if (error instanceof Error) {
    return error.message
  }
  return '未知错误'
}

export default api
