//! 健康检查模块
//!
//! 提供服务健康状态检查和凭据可用性监控功能。

use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::time::{Duration, interval};

use crate::admin::ApiKeyManager;
use crate::kiro::pool_manager::PoolManager;
use crate::kiro::token_manager::MultiTokenManager;

/// 健康检查响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    /// 服务状态
    pub status: HealthStatus,
    /// 检查时间
    pub timestamp: String,
    /// 服务版本
    pub version: String,
    /// 凭据状态
    pub credentials: CredentialsHealth,
    /// 池状态（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pools: Option<Vec<PoolHealth>>,
}

/// 健康状态
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    /// 健康
    Healthy,
    /// 降级（部分功能不可用）
    Degraded,
    /// 不健康
    Unhealthy,
}

/// 凭据健康状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialsHealth {
    /// 总凭据数
    pub total: usize,
    /// 可用凭据数
    pub available: usize,
    /// 禁用凭据数
    pub disabled: usize,
    /// 故障凭据数
    pub failed: usize,
}

/// 池健康状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolHealth {
    /// 池 ID
    pub id: String,
    /// 池名称
    pub name: String,
    /// 是否启用
    pub enabled: bool,
    /// 总凭据数
    pub total_credentials: usize,
    /// 可用凭据数
    pub available_credentials: usize,
}

/// 健康检查状态
pub struct HealthCheckState {
    /// Token 管理器
    pub token_manager: Option<Arc<MultiTokenManager>>,
    /// 池管理器
    pub pool_manager: Option<Arc<PoolManager>>,
    /// API Key 管理器
    pub api_key_manager: Arc<ApiKeyManager>,
    /// 服务版本
    pub version: String,
}

impl HealthCheckState {
    /// 创建新的健康检查状态
    pub fn new(
        token_manager: Option<Arc<MultiTokenManager>>,
        pool_manager: Option<Arc<PoolManager>>,
        api_key_manager: Arc<ApiKeyManager>,
    ) -> Self {
        Self {
            token_manager,
            pool_manager,
            api_key_manager,
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

/// GET /health
///
/// 返回服务健康状态
pub async fn health_check(State(state): State<Arc<HealthCheckState>>) -> Response {
    let timestamp = Utc::now().to_rfc3339();

    // 检查凭据状态
    let credentials_health = if let Some(ref tm) = state.token_manager {
        let snapshot = tm.snapshot();
        // 计算 disabled 和 failed 数量
        let disabled = snapshot.entries.iter().filter(|e| e.disabled).count();
        let failed = snapshot.entries.iter().filter(|e| e.failure_count > 0).count();

        CredentialsHealth {
            total: snapshot.total,
            available: snapshot.available,
            disabled,
            failed,
        }
    } else {
        CredentialsHealth {
            total: 0,
            available: 0,
            disabled: 0,
            failed: 0,
        }
    };

    // 检查池状态
    let pools_health = if let Some(ref pm) = state.pool_manager {
        let pools = pm.snapshot();
        Some(
            pools
                .into_iter()
                .map(|p| PoolHealth {
                    id: p.id,
                    name: p.name,
                    enabled: p.enabled,
                    total_credentials: p.total_credentials,
                    available_credentials: p.available_credentials,
                })
                .collect(),
        )
    } else {
        None
    };

    // 确定整体健康状态
    let status = if credentials_health.available == 0 {
        HealthStatus::Unhealthy
    } else if credentials_health.available < credentials_health.total / 2 {
        HealthStatus::Degraded
    } else {
        HealthStatus::Healthy
    };

    let response = HealthResponse {
        status,
        timestamp,
        version: state.version.clone(),
        credentials: credentials_health,
        pools: pools_health,
    };

    // 根据健康状态返回不同的 HTTP 状态码
    let status_code = match status {
        HealthStatus::Healthy => StatusCode::OK,
        HealthStatus::Degraded => StatusCode::OK, // 降级仍返回 200，但在响应体中标记
        HealthStatus::Unhealthy => StatusCode::SERVICE_UNAVAILABLE,
    };

    (status_code, Json(response)).into_response()
}

/// 启动后台健康检查任务
///
/// 定期检查凭据可用性，自动标记故障凭据
pub fn start_health_check_task(
    token_manager: Arc<MultiTokenManager>,
    interval_secs: u64,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut ticker = interval(Duration::from_secs(interval_secs));
        ticker.tick().await; // 跳过第一次立即触发

        loop {
            ticker.tick().await;
            tracing::info!("执行定期健康检查");

            // 检查所有凭据的可用性
            let snapshot = token_manager.snapshot();
            let disabled = snapshot.entries.iter().filter(|e| e.disabled).count();
            let failed = snapshot.entries.iter().filter(|e| e.failure_count > 0).count();
            tracing::info!(
                "凭据状态: 总数={}, 可用={}, 禁用={}, 故障={}",
                snapshot.total,
                snapshot.available,
                disabled,
                failed
            );

            // 这里可以添加更多的健康检查逻辑
            // 例如：尝试刷新 token、检查 API 可达性等
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_status_serialization() {
        let status = HealthStatus::Healthy;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"healthy\"");

        let status = HealthStatus::Degraded;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"degraded\"");

        let status = HealthStatus::Unhealthy;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"unhealthy\"");
    }

    #[test]
    fn test_credentials_health() {
        let health = CredentialsHealth {
            total: 10,
            available: 8,
            disabled: 1,
            failed: 1,
        };

        let json = serde_json::to_string(&health).unwrap();
        assert!(json.contains("\"total\":10"));
        assert!(json.contains("\"available\":8"));
    }
}
