//! Anthropic API 路由配置

use std::sync::Arc;

use axum::{
    Router,
    extract::DefaultBodyLimit,
    middleware,
    routing::{get, post},
};

use crate::admin::ApiKeyManager;
use crate::kiro::pool_manager::PoolManager;
use crate::kiro::provider::KiroProvider;

use super::{
    handlers::{count_tokens, get_models, post_messages, post_messages_cc},
    middleware::{AppState, auth_middleware, cors_layer},
};

/// 请求体最大大小限制 (50MB)
const MAX_BODY_SIZE: usize = 50 * 1024 * 1024;

/// 创建 Anthropic API 路由
///
/// # 端点
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
/// - `api_key`: API 密钥，用于验证客户端请求
/// - `kiro_provider`: 可选的 KiroProvider，用于调用上游 API

/// 创建带有 KiroProvider 的 Anthropic API 路由
pub fn create_router_with_provider(
    api_key: impl Into<String>,
    kiro_provider: Option<KiroProvider>,
    profile_arn: Option<String>,
) -> Router {
    create_router_full(api_key, kiro_provider, profile_arn, None, None)
}

/// 创建完整的 Anthropic API 路由（支持多 API Key 和池路由）
///
/// # 参数
/// - `api_key`: 静态 API 密钥（后备）
/// - `kiro_provider`: 可选的 KiroProvider（默认池使用）
/// - `profile_arn`: 可选的 Profile ARN
/// - `api_key_manager`: 可选的 API Key 管理器（多 API Key 支持）
/// - `pool_manager`: 可选的池管理器（API Key 绑定池路由）
pub fn create_router_full(
    api_key: impl Into<String>,
    kiro_provider: Option<KiroProvider>,
    profile_arn: Option<String>,
    api_key_manager: Option<Arc<ApiKeyManager>>,
    pool_manager: Option<Arc<PoolManager>>,
) -> Router {
    let mut state = AppState::new(api_key);
    if let Some(provider) = kiro_provider {
        state = state.with_kiro_provider(provider);
    }
    if let Some(arn) = profile_arn {
        state = state.with_profile_arn(arn);
    }
    if let Some(manager) = api_key_manager {
        state = state.with_api_key_manager(manager);
    }
    if let Some(manager) = pool_manager {
        state = state.with_pool_manager(manager);
    }

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

    Router::new()
        .nest("/v1", v1_routes)
        .nest("/cc/v1", cc_v1_routes)
        .layer(cors_layer())
        .layer(DefaultBodyLimit::max(MAX_BODY_SIZE))
        .with_state(state)
}
