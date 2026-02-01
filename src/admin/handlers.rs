//! Admin API HTTP 处理器
//!
//! 提供凭据管理相关的 HTTP 处理器

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};

use super::{
    middleware::AdminState,
    types::{
        AddCredentialRequest, AdminErrorResponse, CsrfTokenResponse, ImportCredentialsRequest,
        SetDisabledRequest, SetPriorityRequest, SetSchedulingModeRequest, SuccessResponse,
    },
};

/// GET /api/admin/csrf-token
/// 获取新的 CSRF Token
pub async fn get_csrf_token(State(state): State<AdminState>) -> impl IntoResponse {
    // 清理过期的 Token
    state.csrf_manager.cleanup_expired();

    let token = state.csrf_manager.generate_token();
    Json(CsrfTokenResponse { token })
}

/// GET /api/admin/credentials
/// 获取所有凭据状态
pub async fn get_all_credentials(State(state): State<AdminState>) -> impl IntoResponse {
    let response = state.service.get_all_credentials();
    Json(response)
}

/// POST /api/admin/credentials/:id/disabled
/// 设置凭据禁用状态
pub async fn set_credential_disabled(
    State(state): State<AdminState>,
    Path(id): Path<u64>,
    Json(payload): Json<SetDisabledRequest>,
) -> impl IntoResponse {
    match state.service.set_disabled(id, payload.disabled) {
        Ok(_) => {
            let action = if payload.disabled { "禁用" } else { "启用" };
            Json(SuccessResponse::new(format!("凭据 #{} 已{}", id, action))).into_response()
        }
        Err(e) => (e.status_code(), Json(e.into_response())).into_response(),
    }
}

/// POST /api/admin/credentials/:id/priority
/// 设置凭据优先级
pub async fn set_credential_priority(
    State(state): State<AdminState>,
    Path(id): Path<u64>,
    Json(payload): Json<SetPriorityRequest>,
) -> impl IntoResponse {
    match state.service.set_priority(id, payload.priority) {
        Ok(_) => Json(SuccessResponse::new(format!(
            "凭据 #{} 优先级已设置为 {}",
            id, payload.priority
        )))
        .into_response(),
        Err(e) => (e.status_code(), Json(e.into_response())).into_response(),
    }
}

/// POST /api/admin/credentials/:id/reset
/// 重置失败计数并重新启用
pub async fn reset_failure_count(
    State(state): State<AdminState>,
    Path(id): Path<u64>,
) -> impl IntoResponse {
    match state.service.reset_and_enable(id) {
        Ok(_) => Json(SuccessResponse::new(format!(
            "凭据 #{} 失败计数已重置并重新启用",
            id
        )))
        .into_response(),
        Err(e) => (e.status_code(), Json(e.into_response())).into_response(),
    }
}

/// GET /api/admin/credentials/:id/balance
/// 获取指定凭据的余额
pub async fn get_credential_balance(
    State(state): State<AdminState>,
    Path(id): Path<u64>,
) -> impl IntoResponse {
    match state.service.get_balance(id).await {
        Ok(response) => Json(response).into_response(),
        Err(e) => (e.status_code(), Json(e.into_response())).into_response(),
    }
}

/// POST /api/admin/credentials
/// 添加新凭据
pub async fn add_credential(
    State(state): State<AdminState>,
    Json(payload): Json<AddCredentialRequest>,
) -> impl IntoResponse {
    match state.service.add_credential(payload).await {
        Ok(response) => Json(response).into_response(),
        Err(e) => (e.status_code(), Json(e.into_response())).into_response(),
    }
}

/// DELETE /api/admin/credentials/:id
/// 删除凭据
pub async fn delete_credential(
    State(state): State<AdminState>,
    Path(id): Path<u64>,
) -> impl IntoResponse {
    match state.service.delete_credential(id) {
        Ok(_) => Json(SuccessResponse::new(format!("凭据 #{} 已删除", id))).into_response(),
        Err(e) => (e.status_code(), Json(e.into_response())).into_response(),
    }
}

/// POST /api/admin/credentials/import
/// 批量导入凭据（支持 IdC 格式）
pub async fn import_credentials(
    State(state): State<AdminState>,
    Json(payload): Json<ImportCredentialsRequest>,
) -> impl IntoResponse {
    if payload.credentials.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(AdminErrorResponse::invalid_request("凭据列表不能为空")),
        )
            .into_response();
    }

    match state.service.import_credentials(payload.credentials, payload.pool_id).await {
        Ok(response) => Json(response).into_response(),
        Err(e) => (e.status_code(), Json(e.into_response())).into_response(),
    }
}

/// POST /api/admin/scheduling-mode
/// 设置调度模式
pub async fn set_scheduling_mode(
    State(state): State<AdminState>,
    Json(payload): Json<SetSchedulingModeRequest>,
) -> impl IntoResponse {
    state.service.set_scheduling_mode(payload.mode);
    let mode_name = match payload.mode {
        crate::kiro::token_manager::SchedulingMode::RoundRobin => "轮询模式",
        crate::kiro::token_manager::SchedulingMode::PriorityFill => "优先填充模式",
    };
    Json(SuccessResponse::new(format!("调度模式已切换为: {}", mode_name)))
}
