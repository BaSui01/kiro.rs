//! Admin API 路由配置

use axum::{
    Router, middleware,
    routing::{delete, get, post, put},
};

use super::{
    api_key_handlers::{create_api_key, delete_api_key, get_api_keys, update_api_key},
    config_handlers::{get_config, update_config},
    handlers::{
        add_credential, delete_credential, get_all_credentials, get_credential_balance,
        get_csrf_token, import_credentials, reset_failure_count, set_credential_disabled,
        set_credential_priority, set_scheduling_mode,
    },
    middleware::{AdminState, admin_auth_middleware, csrf_middleware},
    pool_handlers::{
        assign_credential_to_pool, create_pool, delete_pool, get_all_pools, get_pool,
        get_pool_credentials, set_pool_disabled, update_pool,
    },
};

/// 创建 Admin API 路由
///
/// # 端点
///
/// ## CSRF 保护
/// - `GET /csrf-token` - 获取 CSRF Token（POST/PUT/DELETE 请求需要携带）
///
/// ## 凭据管理
/// - `GET /credentials` - 获取所有凭据状态
/// - `POST /credentials` - 添加新凭据
/// - `POST /credentials/import` - 批量导入凭据（IdC 格式）
/// - `DELETE /credentials/:id` - 删除凭据
/// - `POST /credentials/:id/disabled` - 设置凭据禁用状态
/// - `POST /credentials/:id/priority` - 设置凭据优先级
/// - `POST /credentials/:id/reset` - 重置失败计数
/// - `GET /credentials/:id/balance` - 获取凭据余额
/// - `POST /credentials/:id/pool` - 将凭据分配到池
///
/// ## 调度模式
/// - `POST /scheduling-mode` - 设置调度模式（round_robin / priority_fill）
///
/// ## 池管理
/// - `GET /pools` - 获取所有池
/// - `POST /pools` - 创建新池
/// - `GET /pools/:id` - 获取池详情
/// - `PUT /pools/:id` - 更新池配置
/// - `DELETE /pools/:id` - 删除池
/// - `POST /pools/:id/disabled` - 设置池禁用状态
/// - `GET /pools/:id/credentials` - 获取池的凭证列表
///
/// ## 配置管理
/// - `GET /config` - 获取当前配置
/// - `PUT /config` - 更新配置
///
/// ## API Key 管理
/// - `GET /api-keys` - 获取所有 API Keys
/// - `POST /api-keys` - 创建新 API Key
/// - `PUT /api-keys/:id` - 更新 API Key
/// - `DELETE /api-keys/:id` - 删除 API Key
///
/// # 认证
/// 需要 Admin API Key 认证，支持：
/// - `x-api-key` header
/// - `Authorization: Bearer <token>` header
///
/// # CSRF 保护
/// POST/PUT/DELETE 请求需要携带 `x-csrf-token` 头
pub fn create_admin_router(state: AdminState) -> Router {
    // 需要 CSRF 保护的路由（POST/PUT/DELETE 操作）
    let protected_routes = Router::new()
        // 凭据管理
        .route(
            "/credentials",
            get(get_all_credentials).post(add_credential),
        )
        .route("/credentials/import", post(import_credentials))
        .route("/credentials/{id}", delete(delete_credential))
        .route("/credentials/{id}/disabled", post(set_credential_disabled))
        .route("/credentials/{id}/priority", post(set_credential_priority))
        .route("/credentials/{id}/reset", post(reset_failure_count))
        .route("/credentials/{id}/balance", get(get_credential_balance))
        .route("/credentials/{id}/pool", post(assign_credential_to_pool))
        // 调度模式
        .route("/scheduling-mode", post(set_scheduling_mode))
        // 池管理
        .route("/pools", get(get_all_pools).post(create_pool))
        .route(
            "/pools/{id}",
            get(get_pool).put(update_pool).delete(delete_pool),
        )
        .route("/pools/{id}/disabled", post(set_pool_disabled))
        .route("/pools/{id}/credentials", get(get_pool_credentials))
        // 配置管理
        .route("/config", get(get_config).put(update_config))
        // API Key 管理
        .route("/api-keys", get(get_api_keys).post(create_api_key))
        .route(
            "/api-keys/{id}",
            put(update_api_key).delete(delete_api_key),
        )
        // 应用 CSRF 中间件
        .layer(middleware::from_fn_with_state(
            state.clone(),
            csrf_middleware,
        ));

    // 不需要 CSRF 保护的路由（只有 GET 请求）
    let unprotected_routes = Router::new()
        // CSRF Token 端点（用于获取 Token）
        .route("/csrf-token", get(get_csrf_token));

    // 合并路由并应用认证中间件
    Router::new()
        .merge(protected_routes)
        .merge(unprotected_routes)
        .layer(middleware::from_fn_with_state(
            state.clone(),
            admin_auth_middleware,
        ))
        .with_state(state)
}
