//! Anthropic API 中间件

use std::sync::Arc;

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Json, Response},
};
use dashmap::DashMap;
use std::time::Instant;

use crate::admin::ApiKeyManager;
use crate::kiro::pool_manager::PoolManager;
use crate::kiro::provider::KiroProvider;
use crate::model::config::Config;

use super::types::ErrorResponse;

/// 应用共享状态
#[derive(Clone)]
pub struct AppState {
    /// Kiro Provider（可选，用于实际 API 调用）
    /// 内部使用 MultiTokenManager，已支持线程安全的多凭据管理
    pub kiro_provider: Option<Arc<KiroProvider>>,
    /// Profile ARN（可选，用于请求）
    pub profile_arn: Option<String>,
    /// API Key 管理器（用于 API Key 验证）
    pub api_key_manager: Arc<ApiKeyManager>,
    /// 池管理器（可选，用于 API Key 绑定池路由）
    pub pool_manager: Option<Arc<PoolManager>>,
    /// 限流器（可选）
    pub rate_limiter: Option<Arc<RateLimiter>>,
    /// 应用配置
    pub config: Arc<Config>,
}

impl AppState {
    /// 创建新的应用状态
    pub fn new(api_key_manager: Arc<ApiKeyManager>, config: Arc<Config>) -> Self {
        Self {
            kiro_provider: None,
            profile_arn: None,
            api_key_manager,
            pool_manager: None,
            rate_limiter: None,
            config,
        }
    }

    /// 设置 KiroProvider
    pub fn with_kiro_provider(mut self, provider: KiroProvider) -> Self {
        self.kiro_provider = Some(Arc::new(provider));
        self
    }

    /// 设置 Profile ARN
    pub fn with_profile_arn(mut self, arn: impl Into<String>) -> Self {
        self.profile_arn = Some(arn.into());
        self
    }

    /// 设置池管理器
    pub fn with_pool_manager(mut self, manager: Arc<PoolManager>) -> Self {
        self.pool_manager = Some(manager);
        self
    }

    /// 设置限流器
    pub fn with_rate_limiter(mut self, limiter: Arc<RateLimiter>) -> Self {
        self.rate_limiter = Some(limiter);
        self
    }
}

/// 请求扩展：存储验证后的 pool_id
#[derive(Clone, Debug)]
pub struct AuthenticatedPoolId(pub Option<String>);

/// API Key 认证中间件
///
/// 通过 ApiKeyManager 验证 API Key：
/// - 验证 API Key 是否在 api_keys.json 中且已启用
/// - 提取绑定的 pool_id 并存入请求扩展
pub async fn auth_middleware(
    State(state): State<AppState>,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    use crate::common::auth;

    let key = match auth::extract_api_key(&request) {
        Some(k) => k,
        None => {
            let error = ErrorResponse::authentication_error();
            return (StatusCode::UNAUTHORIZED, Json(error)).into_response();
        }
    };

    // 使用 ApiKeyManager 验证
    if let Some(pool_id) = state.api_key_manager.validate_and_get_pool(&key) {
        // API Key 有效，存储 pool_id 到请求扩展
        request.extensions_mut().insert(AuthenticatedPoolId(pool_id));
        return next.run(request).await;
    }

    // 认证失败
    let error = ErrorResponse::authentication_error();
    (StatusCode::UNAUTHORIZED, Json(error)).into_response()
}

/// CORS 中间件层
///
/// **安全说明**：当前配置允许所有来源（Any），这是为了支持公开 API 服务。
/// 如果需要更严格的安全控制，请根据实际需求配置具体的允许来源、方法和头信息。
///
/// # 配置说明
/// - `allow_origin(Any)`: 允许任何来源的请求
/// - `allow_methods(Any)`: 允许任何 HTTP 方法
/// - `allow_headers(Any)`: 允许任何请求头
pub fn cors_layer() -> tower_http::cors::CorsLayer {
    use tower_http::cors::{Any, CorsLayer};

    CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any)
}

/// 限流器
///
/// 支持全局限流和每 API Key 限流
pub struct RateLimiter {
    /// 全局限流：每分钟请求数
    global_per_minute: u64,
    /// 全局限流：每小时请求数
    global_per_hour: u64,
    /// 每 API Key 限流：每分钟请求数
    per_key_per_minute: u64,
    /// 每 API Key 限流：每小时请求数
    per_key_per_hour: u64,
    /// 全局请求记录（分钟级）
    global_minute_requests: Arc<DashMap<u64, u64>>,
    /// 全局请求记录（小时级）
    global_hour_requests: Arc<DashMap<u64, u64>>,
    /// 每 API Key 请求记录（分钟级）
    key_minute_requests: Arc<DashMap<String, DashMap<u64, u64>>>,
    /// 每 API Key 请求记录（小时级）
    key_hour_requests: Arc<DashMap<String, DashMap<u64, u64>>>,
    /// 启动时间
    start_time: Instant,
}

impl RateLimiter {
    /// 创建新的限流器
    pub fn new(
        global_per_minute: u64,
        global_per_hour: u64,
        per_key_per_minute: u64,
        per_key_per_hour: u64,
    ) -> Self {
        Self {
            global_per_minute,
            global_per_hour,
            per_key_per_minute,
            per_key_per_hour,
            global_minute_requests: Arc::new(DashMap::new()),
            global_hour_requests: Arc::new(DashMap::new()),
            key_minute_requests: Arc::new(DashMap::new()),
            key_hour_requests: Arc::new(DashMap::new()),
            start_time: Instant::now(),
        }
    }

    /// 检查是否允许请求
    ///
    /// 返回 Ok(()) 如果允许，返回 Err(message) 如果被限流
    pub fn check_rate_limit(&self, api_key: Option<&str>) -> Result<(), String> {
        let now = self.start_time.elapsed();
        let current_minute = now.as_secs() / 60;
        let current_hour = now.as_secs() / 3600;

        // 检查全局限流（分钟级）
        let global_minute_count = self
            .global_minute_requests
            .entry(current_minute)
            .or_insert(0)
            .value()
            .clone();

        if global_minute_count >= self.global_per_minute {
            return Err(format!(
                "全局限流：每分钟最多 {} 个请求",
                self.global_per_minute
            ));
        }

        // 检查全局限流（小时级）
        let global_hour_count = self
            .global_hour_requests
            .entry(current_hour)
            .or_insert(0)
            .value()
            .clone();

        if global_hour_count >= self.global_per_hour {
            return Err(format!(
                "全局限流：每小时最多 {} 个请求",
                self.global_per_hour
            ));
        }

        // 检查每 API Key 限流
        if let Some(key) = api_key {
            // 分钟级
            let key_minute_map = self
                .key_minute_requests
                .entry(key.to_string())
                .or_insert_with(DashMap::new);
            let key_minute_count = key_minute_map
                .entry(current_minute)
                .or_insert(0)
                .value()
                .clone();

            if key_minute_count >= self.per_key_per_minute {
                return Err(format!(
                    "API Key 限流：每分钟最多 {} 个请求",
                    self.per_key_per_minute
                ));
            }

            // 小时级
            let key_hour_map = self
                .key_hour_requests
                .entry(key.to_string())
                .or_insert_with(DashMap::new);
            let key_hour_count = key_hour_map
                .entry(current_hour)
                .or_insert(0)
                .value()
                .clone();

            if key_hour_count >= self.per_key_per_hour {
                return Err(format!(
                    "API Key 限流：每小时最多 {} 个请求",
                    self.per_key_per_hour
                ));
            }
        }

        Ok(())
    }

    /// 记录请求
    pub fn record_request(&self, api_key: Option<&str>) {
        let now = self.start_time.elapsed();
        let current_minute = now.as_secs() / 60;
        let current_hour = now.as_secs() / 3600;

        // 记录全局请求
        self.global_minute_requests
            .entry(current_minute)
            .and_modify(|count| *count += 1)
            .or_insert(1);

        self.global_hour_requests
            .entry(current_hour)
            .and_modify(|count| *count += 1)
            .or_insert(1);

        // 记录每 API Key 请求
        if let Some(key) = api_key {
            let key_minute_map = self
                .key_minute_requests
                .entry(key.to_string())
                .or_insert_with(DashMap::new);
            key_minute_map
                .entry(current_minute)
                .and_modify(|count| *count += 1)
                .or_insert(1);

            let key_hour_map = self
                .key_hour_requests
                .entry(key.to_string())
                .or_insert_with(DashMap::new);
            key_hour_map
                .entry(current_hour)
                .and_modify(|count| *count += 1)
                .or_insert(1);
        }

        // 清理过期数据（保留最近 2 小时的数据）
        self.cleanup_old_records(current_minute, current_hour);
    }

    /// 清理过期记录
    fn cleanup_old_records(&self, current_minute: u64, current_hour: u64) {
        // 清理超过 2 小时的分钟级记录
        let minute_threshold = current_minute.saturating_sub(120);
        self.global_minute_requests
            .retain(|&k, _| k > minute_threshold);

        // 清理超过 2 小时的小时级记录
        let hour_threshold = current_hour.saturating_sub(2);
        self.global_hour_requests.retain(|&k, _| k > hour_threshold);

        // 清理每 API Key 的过期记录
        for entry in self.key_minute_requests.iter_mut() {
            entry.value().retain(|&k, _| k > minute_threshold);
        }

        for entry in self.key_hour_requests.iter_mut() {
            entry.value().retain(|&k, _| k > hour_threshold);
        }
    }
}

/// 限流中间件
///
/// 检查请求是否超过限流阈值，如果超过则返回 429 Too Many Requests
pub async fn rate_limit_middleware(
    State(state): State<AppState>,
    request: Request<Body>,
    next: Next,
) -> Response {
    // 如果没有配置限流器，直接放行
    let limiter = match &state.rate_limiter {
        Some(l) => l,
        None => return next.run(request).await,
    };

    // 提取 API Key（如果有）
    let api_key = crate::common::auth::extract_api_key(&request);

    // 检查限流
    if let Err(message) = limiter.check_rate_limit(api_key.as_deref()) {
        tracing::warn!("限流触发: {}", message);
        let error = ErrorResponse::new("rate_limit_error", &message);
        return (StatusCode::TOO_MANY_REQUESTS, Json(error)).into_response();
    }

    // 记录请求
    limiter.record_request(api_key.as_deref());

    next.run(request).await
}
