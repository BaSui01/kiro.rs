//! Admin API 中间件

use parking_lot::RwLock;
use std::path::PathBuf;
use std::sync::Arc;

use axum::{
    body::Body,
    extract::State,
    http::{Method, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Json, Response},
};

use super::api_keys::ApiKeyManager;
use super::csrf::CsrfManager;
use super::service::AdminService;
use super::types::AdminErrorResponse;
use crate::common::auth;
use crate::kiro::pool_manager::PoolManager;
use crate::model::config::Config;

/// Admin API 共享状态
#[derive(Clone)]
pub struct AdminState {
    /// Admin API 密钥
    pub admin_api_key: String,
    /// Admin 服务
    pub service: Arc<AdminService>,
    /// 配置（可修改）
    pub config: Arc<RwLock<Config>>,
    /// 配置文件路径
    pub config_path: PathBuf,
    /// API Key 管理器
    pub api_key_manager: Arc<ApiKeyManager>,
    /// 池管理器（可选，用于池管理功能）
    pub pool_manager: Option<Arc<PoolManager>>,
    /// CSRF 管理器
    pub csrf_manager: Arc<CsrfManager>,
}

impl AdminState {
    pub fn new(
        admin_api_key: impl Into<String>,
        service: AdminService,
        config: Config,
        config_path: impl Into<PathBuf>,
        api_key_manager: Arc<ApiKeyManager>,
    ) -> Self {
        Self {
            admin_api_key: admin_api_key.into(),
            service: Arc::new(service),
            config: Arc::new(RwLock::new(config)),
            config_path: config_path.into(),
            api_key_manager,
            pool_manager: None,
            // CSRF Token 有效期：1 小时
            csrf_manager: Arc::new(CsrfManager::new(3600)),
        }
    }

    /// 设置池管理器
    pub fn with_pool_manager(mut self, pool_manager: Arc<PoolManager>) -> Self {
        self.pool_manager = Some(pool_manager);
        self
    }

    /// 获取配置的克隆
    pub fn get_config(&self) -> Config {
        self.config.read().clone()
    }

    /// 更新配置并持久化
    pub fn update_config<F>(&self, updater: F) -> anyhow::Result<Config>
    where
        F: FnOnce(&mut Config),
    {
        let mut config = self.config.write();
        updater(&mut config);
        config.save(&self.config_path)?;
        Ok(config.clone())
    }
}

/// Admin API 认证中间件
pub async fn admin_auth_middleware(
    State(state): State<AdminState>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let api_key = auth::extract_api_key(&request);

    match api_key {
        Some(key) if auth::constant_time_eq(&key, &state.admin_api_key) => next.run(request).await,
        _ => {
            let error = AdminErrorResponse::authentication_error();
            (StatusCode::UNAUTHORIZED, Json(error)).into_response()
        }
    }
}

/// CSRF 验证中间件
///
/// 对 POST/PUT/DELETE 请求验证 x-csrf-token 头
pub async fn csrf_middleware(
    State(state): State<AdminState>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let method = request.method().clone();

    // 只对修改操作验证 CSRF Token
    if method == Method::POST || method == Method::PUT || method == Method::DELETE {
        // 获取 CSRF Token
        let csrf_token = request
            .headers()
            .get("x-csrf-token")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        match csrf_token {
            Some(token) if state.csrf_manager.validate_token(&token) => {
                next.run(request).await
            }
            _ => {
                let error = AdminErrorResponse::new("csrf_error", "Invalid or missing CSRF token");
                (StatusCode::FORBIDDEN, Json(error)).into_response()
            }
        }
    } else {
        // GET 等只读请求不需要 CSRF 验证
        next.run(request).await
    }
}
