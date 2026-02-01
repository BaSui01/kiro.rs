//! API Key 管理 HTTP 处理器
//!
//! 提供 API Key 的 CRUD 操作功能

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};

use super::{
    api_keys::{ApiKeyError, CreateApiKeyRequest, UpdateApiKeyRequest},
    middleware::AdminState,
    types::{AdminErrorResponse, SuccessResponse},
};

/// GET /api/admin/api-keys
/// 获取所有 API Keys
pub async fn get_api_keys(State(state): State<AdminState>) -> impl IntoResponse {
    let keys = state.api_key_manager.list();
    Json(keys)
}

/// POST /api/admin/api-keys
/// 创建新 API Key
pub async fn create_api_key(
    State(state): State<AdminState>,
    Json(payload): Json<CreateApiKeyRequest>,
) -> impl IntoResponse {
    match state.api_key_manager.create_with_full_key(payload) {
        Ok(key) => (StatusCode::CREATED, Json(key)).into_response(),
        Err(e) => match e {
            ApiKeyError::DuplicateName(_) => (
                StatusCode::CONFLICT,
                Json(AdminErrorResponse::invalid_request(e.to_string())),
            )
                .into_response(),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AdminErrorResponse::internal_error(format!("创建 API Key 失败: {}", e))),
            )
                .into_response(),
        },
    }
}

/// PUT /api/admin/api-keys/:id
/// 更新 API Key
pub async fn update_api_key(
    State(state): State<AdminState>,
    Path(id): Path<u64>,
    Json(payload): Json<UpdateApiKeyRequest>,
) -> impl IntoResponse {
    match state.api_key_manager.update(id, payload) {
        Ok(key) => Json(key).into_response(),
        Err(e) => match e {
            ApiKeyError::NotFound(_) => (
                StatusCode::NOT_FOUND,
                Json(AdminErrorResponse::not_found(e.to_string())),
            )
                .into_response(),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AdminErrorResponse::internal_error(e.to_string())),
            )
                .into_response(),
        },
    }
}

/// DELETE /api/admin/api-keys/:id
/// 删除 API Key
pub async fn delete_api_key(
    State(state): State<AdminState>,
    Path(id): Path<u64>,
) -> impl IntoResponse {
    match state.api_key_manager.delete(id) {
        Ok(_) => Json(SuccessResponse::new(format!("API Key #{} 已删除", id))).into_response(),
        Err(e) => match e {
            ApiKeyError::NotFound(_) => (
                StatusCode::NOT_FOUND,
                Json(AdminErrorResponse::not_found(e.to_string())),
            )
                .into_response(),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AdminErrorResponse::internal_error(e.to_string())),
            )
                .into_response(),
        },
    }
}
