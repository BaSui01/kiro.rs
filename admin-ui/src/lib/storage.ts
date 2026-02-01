/**
 * 安全的 API Key 和 CSRF Token 存储模块
 *
 * 安全改进：
 * 1. 使用 sessionStorage 替代 localStorage（关闭浏览器后自动清除）
 * 2. 内存缓存优先（减少存储访问）
 * 3. 自动过期机制（8小时后需要重新登录）
 * 4. CSRF Token 存储（用于防止跨站请求伪造）
 * 5. 用户活动监听，自动刷新会话有效期
 */

const API_KEY_STORAGE_KEY = 'adminApiKey'
const API_KEY_EXPIRY_KEY = 'adminApiKey_expiry'
const CSRF_TOKEN_STORAGE_KEY = 'csrfToken'

// 过期时间：8小时
const EXPIRY_DURATION_MS = 8 * 60 * 60 * 1000

// 活动刷新间隔：30分钟（避免频繁更新）
const ACTIVITY_REFRESH_INTERVAL_MS = 30 * 60 * 1000

// 内存缓存（优先使用，避免频繁访问 sessionStorage）
let memoryCache: string | null = null
let csrfTokenCache: string | null = null

// 上次活动刷新时间
let lastActivityRefresh = 0

// 活动监听器是否已初始化
let activityListenerInitialized = false

// 保存事件处理函数引用，以便后续移除
let activityHandler: (() => void) | null = null

export const storage = {
  /**
   * 获取 API Key
   *
   * 优先级：
   * 1. 内存缓存
   * 2. sessionStorage（检查过期）
   */
  getApiKey: (): string | null => {
    // 优先从内存获取
    if (memoryCache) {
      return memoryCache
    }

    // 检查是否过期
    const expiry = sessionStorage.getItem(API_KEY_EXPIRY_KEY)
    if (expiry) {
      const expiryTime = parseInt(expiry, 10)
      // 验证是否为有效数字
      if (isNaN(expiryTime) || Date.now() > expiryTime) {
        // 已过期或无效，清除存储
        storage.removeApiKey()
        return null
      }
    }

    // 从 sessionStorage 获取
    const key = sessionStorage.getItem(API_KEY_STORAGE_KEY)
    if (key) {
      // 缓存到内存
      memoryCache = key
    }
    return key
  },

  /**
   * 设置 API Key
   *
   * 同时存储到内存和 sessionStorage，并设置过期时间
   */
  setApiKey: (key: string): void => {
    // 存储到内存
    memoryCache = key

    // 存储到 sessionStorage
    sessionStorage.setItem(API_KEY_STORAGE_KEY, key)

    // 设置过期时间
    const expiryTime = Date.now() + EXPIRY_DURATION_MS
    sessionStorage.setItem(API_KEY_EXPIRY_KEY, String(expiryTime))

    // 初始化活动监听器
    storage.initActivityListener()
  },

  /**
   * 移除 API Key
   *
   * 清除内存缓存和 sessionStorage，同时移除活动监听器
   */
  removeApiKey: (): void => {
    memoryCache = null
    sessionStorage.removeItem(API_KEY_STORAGE_KEY)
    sessionStorage.removeItem(API_KEY_EXPIRY_KEY)
    // 同时清除 CSRF Token
    storage.removeCsrfToken()
    // 移除活动监听器（避免资源泄漏）
    storage.removeActivityListener()
  },

  /**
   * 检查 API Key 是否已过期
   */
  isExpired: (): boolean => {
    const expiry = sessionStorage.getItem(API_KEY_EXPIRY_KEY)
    if (!expiry) {
      return true
    }
    const expiryTime = parseInt(expiry, 10)
    if (isNaN(expiryTime)) {
      return true
    }
    return Date.now() > expiryTime
  },

  /**
   * 获取剩余有效时间（毫秒）
   *
   * 返回 0 表示已过期或未设置
   */
  getRemainingTime: (): number => {
    const expiry = sessionStorage.getItem(API_KEY_EXPIRY_KEY)
    if (!expiry) {
      return 0
    }
    const expiryTime = parseInt(expiry, 10)
    if (isNaN(expiryTime)) {
      return 0
    }
    const remaining = expiryTime - Date.now()
    return Math.max(0, remaining)
  },

  /**
   * 刷新过期时间
   *
   * 在用户活跃时调用，延长会话有效期
   */
  refreshExpiry: (): void => {
    const key = storage.getApiKey()
    if (key) {
      const expiryTime = Date.now() + EXPIRY_DURATION_MS
      sessionStorage.setItem(API_KEY_EXPIRY_KEY, String(expiryTime))
    }
  },

  /**
   * 初始化用户活动监听器
   *
   * 监听用户活动（鼠标移动、键盘输入、点击等），自动刷新会话有效期
   */
  initActivityListener: (): void => {
    if (activityListenerInitialized) {
      return
    }

    activityHandler = () => {
      const now = Date.now()
      // 限制刷新频率，避免频繁更新
      if (now - lastActivityRefresh > ACTIVITY_REFRESH_INTERVAL_MS) {
        lastActivityRefresh = now
        storage.refreshExpiry()
      }
    }

    // 监听用户活动事件
    const events = ['mousedown', 'keydown', 'touchstart', 'scroll']
    events.forEach((event) => {
      window.addEventListener(event, activityHandler!, { passive: true })
    })

    activityListenerInitialized = true
  },

  /**
   * 移除用户活动监听器
   *
   * 在用户登出或 API Key 被移除时调用，避免资源泄漏
   */
  removeActivityListener: (): void => {
    if (!activityListenerInitialized || !activityHandler) {
      return
    }

    const events = ['mousedown', 'keydown', 'touchstart', 'scroll']
    events.forEach((event) => {
      window.removeEventListener(event, activityHandler!)
    })

    activityListenerInitialized = false
    activityHandler = null
    lastActivityRefresh = 0
  },

  // ============ CSRF Token 管理 ============

  /**
   * 获取 CSRF Token
   *
   * 优先从内存缓存获取
   */
  getCsrfToken: (): string | null => {
    // 优先从内存获取
    if (csrfTokenCache) {
      return csrfTokenCache
    }

    // 从 sessionStorage 获取
    const token = sessionStorage.getItem(CSRF_TOKEN_STORAGE_KEY)
    if (token) {
      csrfTokenCache = token
    }
    return token
  },

  /**
   * 设置 CSRF Token
   *
   * 同时存储到内存和 sessionStorage
   */
  setCsrfToken: (token: string): void => {
    csrfTokenCache = token
    sessionStorage.setItem(CSRF_TOKEN_STORAGE_KEY, token)
  },

  /**
   * 移除 CSRF Token
   *
   * 清除内存缓存和 sessionStorage
   */
  removeCsrfToken: (): void => {
    csrfTokenCache = null
    sessionStorage.removeItem(CSRF_TOKEN_STORAGE_KEY)
  },
}
