//! 配置管理 HTTP 处理器
//!
//! 提供系统配置的读取和更新功能

use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::IntoResponse,
};

use super::{
    middleware::AdminState,
    types::{AdminErrorResponse, ConfigResponse, SuccessResponse, UpdateConfigRequest},
};

/// GET /api/admin/config
/// 获取当前配置
pub async fn get_config(State(state): State<AdminState>) -> impl IntoResponse {
    let config = state.get_config();

    let response = ConfigResponse {
        host: config.host,
        port: config.port,
        region: config.region,
        kiro_version: config.kiro_version,
        tls_backend: config.tls_backend,
        session_cache_max_capacity: config.session_cache_max_capacity,
        session_cache_ttl_secs: config.session_cache_ttl_secs,
        proxy_url: config.proxy_url,
        proxy_username: config.proxy_username,
        // 脱敏代理密码
        proxy_password: config.proxy_password.map(|_| "***".to_string()),
        has_api_key: config.api_key.is_some(),
        has_admin_api_key: config.admin_api_key.is_some(),
    };

    Json(response)
}

/// PUT /api/admin/config
/// 更新配置
pub async fn update_config(
    State(state): State<AdminState>,
    Json(payload): Json<UpdateConfigRequest>,
) -> impl IntoResponse {
    match state.update_config(|config| {
        if let Some(host) = payload.host {
            config.host = host;
        }
        if let Some(port) = payload.port {
            config.port = port;
        }
        if let Some(region) = payload.region {
            config.region = region;
        }
        if let Some(capacity) = payload.session_cache_max_capacity {
            config.session_cache_max_capacity = capacity;
        }
        if let Some(ttl) = payload.session_cache_ttl_secs {
            config.session_cache_ttl_secs = ttl;
        }
        if let Some(proxy_url) = payload.proxy_url {
            config.proxy_url = if proxy_url.is_empty() {
                None
            } else {
                Some(proxy_url)
            };
        }
        if let Some(proxy_username) = payload.proxy_username {
            config.proxy_username = if proxy_username.is_empty() {
                None
            } else {
                Some(proxy_username)
            };
        }
        // 代理密码：空字符串表示不修改，特殊值 "__CLEAR__" 表示清空
        if let Some(proxy_password) = payload.proxy_password {
            if proxy_password == "__CLEAR__" {
                config.proxy_password = None;
            } else if !proxy_password.is_empty() {
                config.proxy_password = Some(proxy_password);
            }
            // 空字符串：不修改
        }
        // API Key：空字符串表示不修改，特殊值 "__CLEAR__" 表示清空
        if let Some(api_key) = payload.api_key {
            if api_key == "__CLEAR__" {
                config.api_key = None;
            } else if !api_key.is_empty() {
                config.api_key = Some(api_key);
            }
            // 空字符串：不修改
        }
    }) {
        Ok(_) => Json(SuccessResponse::new("配置已更新，部分配置需要重启服务后生效")).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(AdminErrorResponse::internal_error(format!("保存配置失败: {}", e))),
        )
            .into_response(),
    }
}
