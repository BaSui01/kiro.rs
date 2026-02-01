//! 池管理 HTTP 处理器
//!
//! 提供凭据池的 CRUD 操作和状态管理功能

use axum::{
    Json,
    body::Body,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::kiro::pool::{Pool, PoolError};
use crate::kiro::pool_manager::UpdatePoolRequest as PoolUpdateRequest;

use super::{
    middleware::AdminState,
    types::{
        AdminErrorResponse, AssignCredentialToPoolRequest, CreatePoolRequest, PoolStatusItem,
        PoolsListResponse, SetPoolDisabledRequest, SuccessResponse, UpdatePoolRequest,
    },
};

/// 将 PoolError 转换为 HTTP 响应
fn pool_error_to_response(e: PoolError) -> Response<Body> {
    match &e {
        PoolError::PoolNotFound { .. } => (
            StatusCode::NOT_FOUND,
            Json(AdminErrorResponse::not_found(e.to_string())),
        )
            .into_response(),
        PoolError::PoolAlreadyExists { .. } => (
            StatusCode::CONFLICT,
            Json(AdminErrorResponse::invalid_request(e.to_string())),
        )
            .into_response(),
        PoolError::CannotDeleteDefaultPool => (
            StatusCode::BAD_REQUEST,
            Json(AdminErrorResponse::invalid_request(e.to_string())),
        )
            .into_response(),
        PoolError::CredentialNotFound { .. } => (
            StatusCode::NOT_FOUND,
            Json(AdminErrorResponse::not_found(e.to_string())),
        )
            .into_response(),
        PoolError::ConfigLoadFailed { .. } => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(AdminErrorResponse::internal_error(e.to_string())),
        )
            .into_response(),
        PoolError::IoError(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(AdminErrorResponse::internal_error(e.to_string())),
        )
            .into_response(),
        PoolError::JsonError(_) => (
            StatusCode::BAD_REQUEST,
            Json(AdminErrorResponse::invalid_request(e.to_string())),
        )
            .into_response(),
        PoolError::TokenManagerError(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(AdminErrorResponse::internal_error(e.to_string())),
        )
            .into_response(),
    }
}

/// GET /api/admin/pools
/// 获取所有池
pub async fn get_all_pools(State(state): State<AdminState>) -> impl IntoResponse {
    match &state.pool_manager {
        Some(pm) => {
            let pools = pm.snapshot();
            Json(PoolsListResponse {
                pools: pools
                    .into_iter()
                    .map(|p| PoolStatusItem {
                        id: p.id,
                        name: p.name,
                        description: p.description,
                        enabled: p.enabled,
                        scheduling_mode: p.scheduling_mode,
                        has_proxy: p.has_proxy,
                        priority: p.priority,
                        total_credentials: p.total_credentials,
                        available_credentials: p.available_credentials,
                        current_id: p.current_id,
                        session_cache_size: p.session_cache_size,
                        round_robin_counter: p.round_robin_counter,
                    })
                    .collect(),
            })
            .into_response()
        }
        None => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(AdminErrorResponse::api_error("池管理器未初始化")),
        )
            .into_response(),
    }
}

/// POST /api/admin/pools
/// 创建新池
pub async fn create_pool(
    State(state): State<AdminState>,
    Json(payload): Json<CreatePoolRequest>,
) -> impl IntoResponse {
    match &state.pool_manager {
        Some(pm) => {
            let pool = Pool::new(&payload.id, &payload.name)
                .with_scheduling_mode(payload.scheduling_mode)
                .with_priority(payload.priority);

            let pool = if let Some(desc) = payload.description {
                pool.with_description(desc)
            } else {
                pool
            };

            let pool = if let Some(proxy_url) = payload.proxy_url {
                pool.with_proxy(proxy_url, payload.proxy_username, payload.proxy_password)
            } else {
                pool
            };

            match pm.create_pool(pool) {
                Ok(_) => (
                    StatusCode::CREATED,
                    Json(SuccessResponse::new(format!("池 {} 创建成功", payload.id))),
                )
                    .into_response(),
                Err(e) => pool_error_to_response(e),
            }
        }
        None => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(AdminErrorResponse::api_error("池管理器未初始化")),
        )
            .into_response(),
    }
}

/// GET /api/admin/pools/:id
/// 获取池详情
pub async fn get_pool(
    State(state): State<AdminState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match &state.pool_manager {
        Some(pm) => match pm.get_pool(&id) {
            Some(pool) => {
                let snapshot = pool.token_manager.snapshot();
                Json(PoolStatusItem {
                    id: pool.config.id.clone(),
                    name: pool.config.name.clone(),
                    description: pool.config.description.clone(),
                    enabled: pool.config.enabled,
                    scheduling_mode: pool.config.scheduling_mode,
                    has_proxy: pool.config.has_proxy(),
                    priority: pool.config.priority,
                    total_credentials: snapshot.total,
                    available_credentials: snapshot.available,
                    current_id: snapshot.current_id,
                    session_cache_size: snapshot.session_cache_size as u64,
                    round_robin_counter: snapshot.round_robin_counter,
                })
                .into_response()
            }
            None => (
                StatusCode::NOT_FOUND,
                Json(AdminErrorResponse::not_found(format!("池不存在: {}", id))),
            )
                .into_response(),
        },
        None => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(AdminErrorResponse::api_error("池管理器未初始化")),
        )
            .into_response(),
    }
}

/// PUT /api/admin/pools/:id
/// 更新池配置
pub async fn update_pool(
    State(state): State<AdminState>,
    Path(id): Path<String>,
    Json(payload): Json<UpdatePoolRequest>,
) -> impl IntoResponse {
    match &state.pool_manager {
        Some(pm) => {
            let updates = PoolUpdateRequest {
                name: payload.name,
                description: payload.description,
                enabled: payload.enabled,
                scheduling_mode: payload.scheduling_mode,
                proxy_url: payload.proxy_url,
                proxy_username: payload.proxy_username,
                proxy_password: payload.proxy_password,
                priority: payload.priority,
            };

            match pm.update_pool(&id, updates) {
                Ok(_) => Json(SuccessResponse::new(format!("池 {} 已更新", id))).into_response(),
                Err(e) => pool_error_to_response(e),
            }
        }
        None => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(AdminErrorResponse::api_error("池管理器未初始化")),
        )
            .into_response(),
    }
}

/// DELETE /api/admin/pools/:id
/// 删除池
pub async fn delete_pool(
    State(state): State<AdminState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match &state.pool_manager {
        Some(pm) => match pm.delete_pool(&id) {
            Ok(_) => Json(SuccessResponse::new(format!("池 {} 已删除", id))).into_response(),
            Err(e) => pool_error_to_response(e),
        },
        None => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(AdminErrorResponse::api_error("池管理器未初始化")),
        )
            .into_response(),
    }
}

/// POST /api/admin/pools/:id/disabled
/// 设置池禁用状态
pub async fn set_pool_disabled(
    State(state): State<AdminState>,
    Path(id): Path<String>,
    Json(payload): Json<SetPoolDisabledRequest>,
) -> impl IntoResponse {
    match &state.pool_manager {
        Some(pm) => match pm.set_pool_disabled(&id, payload.disabled) {
            Ok(_) => {
                let action = if payload.disabled { "禁用" } else { "启用" };
                Json(SuccessResponse::new(format!("池 {} 已{}", id, action))).into_response()
            }
            Err(e) => pool_error_to_response(e),
        },
        None => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(AdminErrorResponse::api_error("池管理器未初始化")),
        )
            .into_response(),
    }
}

/// POST /api/admin/credentials/:id/pool
/// 将凭据分配到池
pub async fn assign_credential_to_pool(
    State(state): State<AdminState>,
    Path(id): Path<u64>,
    Json(payload): Json<AssignCredentialToPoolRequest>,
) -> impl IntoResponse {
    match &state.pool_manager {
        Some(pm) => match pm.assign_credential_to_pool(id, &payload.pool_id) {
            Ok(_) => Json(SuccessResponse::new(format!(
                "凭据 #{} 已分配到池 {}",
                id, payload.pool_id
            )))
            .into_response(),
            Err(e) => pool_error_to_response(e),
        },
        None => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(AdminErrorResponse::api_error("池管理器未初始化")),
        )
            .into_response(),
    }
}
