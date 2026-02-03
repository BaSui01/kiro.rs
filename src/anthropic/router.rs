//! Anthropic API 路由配置

use std::sync::Arc;

use axum::{
    Router,
    extract::DefaultBodyLimit,
    middleware,
    routing::{get, post},
};

use crate::admin::ApiKeyManager;
use crate::health::HealthCheckState;
use crate::kiro::pool_manager::PoolManager;
use crate::kiro::provider::KiroProvider;
use crate::kiro::token_manager::MultiTokenManager;

use super::{
    handlers::{count_tokens, get_models, post_messages, post_messages_cc},
    middleware::{AppState, RateLimiter, auth_middleware, cors_layer, rate_limit_middleware},
};

/// 请求体最大大小限制 (50MB)
const MAX_BODY_SIZE: usize = 50 * 1024 * 1024;

/// 创建 Anthropic API 路由
///
/// # 端点
/// - `GET /health` - 健康检查
/// - `GET /v1/models` - 获取可用模型列表
/// - `POST /v1/messages` - 创建消息（对话）
/// - `POST /v1/messages/count_tokens` - 计算 token 数量
///
/// # 认证
/// 所有 `/v1` 路径需要 API Key 认证，支持：
/// - `x-api-key` header
/// - `Authorization: Bearer <token>` header
///
/// # 参数
/// - `api_key_manager`: API Key 管理器，用于验证客户端请求
/// - `kiro_provider`: 可选的 KiroProvider，用于调用上游 API
/// - `profile_arn`: 可选的 Profile ARN
/// - `pool_manager`: 可选的池管理器（API Key 绑定池路由）
/// - `token_manager`: 可选的 Token 管理器（用于健康检查）
/// - `config`: 应用配置
pub fn create_router(
    api_key_manager: Arc<ApiKeyManager>,
    kiro_provider: Option<KiroProvider>,
    profile_arn: Option<String>,
    pool_manager: Option<Arc<PoolManager>>,
    token_manager: Option<Arc<MultiTokenManager>>,
    config: Arc<crate::model::config::Config>,
) -> Router {
    let mut state = AppState::new(api_key_manager.clone(), config.clone());
    if let Some(provider) = kiro_provider {
        state = state.with_kiro_provider(provider);
    }
    if let Some(arn) = profile_arn {
        state = state.with_profile_arn(arn);
    }
    if let Some(manager) = pool_manager.clone() {
        state = state.with_pool_manager(manager);
    }

    // 配置限流器
    if config.rate_limit_enabled {
        let limiter = Arc::new(RateLimiter::new(
            config.rate_limit_per_minute,
            config.rate_limit_per_hour,
            config.rate_limit_per_key_per_minute,
            config.rate_limit_per_key_per_hour,
        ));
        state = state.with_rate_limiter(limiter);
    }

    // 创建健康检查状态
    let health_state = Arc::new(HealthCheckState::new(
        token_manager,
        pool_manager,
        api_key_manager,
    ));

    // 需要认证的 /v1 路由
    let v1_routes = Router::new()
        .route("/models", get(get_models))
        .route("/messages", post(post_messages))
        .route("/messages/count_tokens", post(count_tokens))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ));

    // 需要认证的 /cc/v1 路由（Claude Code 兼容端点）
    // 与 /v1 的区别：流式响应会等待 contextUsageEvent 后再发送 message_start
    let cc_v1_routes = Router::new()
        .route("/messages", post(post_messages_cc))
        .route("/messages/count_tokens", post(count_tokens))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ));

    let mut router = Router::new()
        .route("/health", get(crate::health::health_check))
        .with_state(health_state)
        .nest("/v1", v1_routes)
        .nest("/cc/v1", cc_v1_routes)
        .layer(cors_layer())
        .layer(DefaultBodyLimit::max(MAX_BODY_SIZE))
        .with_state(state.clone());

    // 添加限流中间件（如果启用）
    if config.rate_limit_enabled {
        router = router.layer(middleware::from_fn_with_state(
            state,
            rate_limit_middleware,
        ));
    }

    router
}
