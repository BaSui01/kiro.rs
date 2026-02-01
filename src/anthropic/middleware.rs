//! Anthropic API 中间件

use std::sync::Arc;

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Json, Response},
};

use crate::admin::ApiKeyManager;
use crate::kiro::pool_manager::PoolManager;
use crate::kiro::provider::KiroProvider;

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
}

impl AppState {
    /// 创建新的应用状态
    pub fn new(api_key_manager: Arc<ApiKeyManager>) -> Self {
        Self {
            kiro_provider: None,
            profile_arn: None,
            api_key_manager,
            pool_manager: None,
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
