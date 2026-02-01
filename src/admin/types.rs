//! Admin API 类型定义

use serde::{Deserialize, Serialize};

use crate::kiro::token_manager::SchedulingMode;
use crate::model::config::TlsBackend;

// ============ 凭据状态 ============

/// 所有凭据状态响应
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CredentialsStatusResponse {
    /// 凭据总数
    pub total: usize,
    /// 可用凭据数量（未禁用）
    pub available: usize,
    /// 当前活跃凭据 ID
    pub current_id: u64,
    /// 各凭据状态列表
    pub credentials: Vec<CredentialStatusItem>,
    /// 会话缓存大小
    pub session_cache_size: usize,
    /// 轮询计数器
    pub round_robin_counter: u64,
    /// 当前调度模式
    pub scheduling_mode: SchedulingMode,
}

/// 单个凭据的状态信息
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CredentialStatusItem {
    /// 凭据唯一 ID
    pub id: u64,
    /// 优先级（数字越小优先级越高）
    pub priority: u32,
    /// 是否被禁用
    pub disabled: bool,
    /// 连续失败次数
    pub failure_count: u32,
    /// 是否为当前活跃凭据
    pub is_current: bool,
    /// Token 过期时间（RFC3339 格式）
    pub expires_at: Option<String>,
    /// 认证方式
    pub auth_method: Option<String>,
    /// 是否有 Profile ARN
    pub has_profile_arn: bool,
}

// ============ 操作请求 ============

/// 启用/禁用凭据请求
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetDisabledRequest {
    /// 是否禁用
    pub disabled: bool,
}

/// 修改优先级请求
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetPriorityRequest {
    /// 新优先级值
    pub priority: u32,
}

/// 设置调度模式请求
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetSchedulingModeRequest {
    /// 调度模式: "round_robin" 或 "priority_fill"
    pub mode: SchedulingMode,
}

/// 添加凭据请求
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddCredentialRequest {
    /// 刷新令牌（必填）
    pub refresh_token: String,

    /// 认证方式（可选，默认 social）
    #[serde(default = "default_auth_method")]
    pub auth_method: String,

    /// OIDC Client ID（IdC 认证需要）
    pub client_id: Option<String>,

    /// OIDC Client Secret（IdC 认证需要）
    pub client_secret: Option<String>,

    /// 优先级（可选，默认 0）
    #[serde(default)]
    pub priority: u32,

    /// 凭据级 Region 配置（用于 OIDC token 刷新）
    /// 未配置时回退到 config.json 的全局 region
    pub region: Option<String>,

    /// 凭据级 Machine ID（可选，64 位字符串）
    /// 未配置时回退到 config.json 的 machineId
    pub machine_id: Option<String>,

    /// 所属池 ID（未配置时归入默认池）
    pub pool_id: Option<String>,

    /// 凭据级代理 URL（优先级高于池级和全局代理）
    pub proxy_url: Option<String>,

    /// 凭据级代理用户名
    pub proxy_username: Option<String>,

    /// 凭据级代理密码
    pub proxy_password: Option<String>,
}

fn default_auth_method() -> String {
    "social".to_string()
}

/// 添加凭据成功响应
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddCredentialResponse {
    pub success: bool,
    pub message: String,
    /// 新添加的凭据 ID
    pub credential_id: u64,
}

// ============ 余额查询 ============

/// 余额查询响应
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BalanceResponse {
    /// 凭据 ID
    pub id: u64,
    /// 订阅类型
    pub subscription_title: Option<String>,
    /// 当前使用量
    pub current_usage: f64,
    /// 使用限额
    pub usage_limit: f64,
    /// 剩余额度
    pub remaining: f64,
    /// 使用百分比
    pub usage_percentage: f64,
    /// 下次重置时间（Unix 时间戳）
    pub next_reset_at: Option<f64>,
}

// ============ 通用响应 ============

/// 操作成功响应
#[derive(Debug, Serialize)]
pub struct SuccessResponse {
    pub success: bool,
    pub message: String,
}

impl SuccessResponse {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            success: true,
            message: message.into(),
        }
    }
}

/// CSRF Token 响应
#[derive(Debug, Serialize)]
pub struct CsrfTokenResponse {
    /// CSRF Token
    pub token: String,
}

/// 错误响应
#[derive(Debug, Serialize)]
pub struct AdminErrorResponse {
    pub error: AdminError,
}

#[derive(Debug, Serialize)]
pub struct AdminError {
    #[serde(rename = "type")]
    pub error_type: String,
    pub message: String,
}

impl AdminErrorResponse {
    pub fn new(error_type: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            error: AdminError {
                error_type: error_type.into(),
                message: message.into(),
            },
        }
    }

    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self::new("invalid_request", message)
    }

    pub fn authentication_error() -> Self {
        Self::new("authentication_error", "Invalid or missing admin API key")
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new("not_found", message)
    }

    pub fn api_error(message: impl Into<String>) -> Self {
        Self::new("api_error", message)
    }

    pub fn internal_error(message: impl Into<String>) -> Self {
        Self::new("internal_error", message)
    }
}

// ============ 配置管理 ============

/// 配置响应（脱敏）
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigResponse {
    /// 服务器地址
    pub host: String,
    /// 服务器端口
    pub port: u16,
    /// AWS Region
    pub region: String,
    /// Kiro 版本
    pub kiro_version: String,
    /// TLS 后端
    pub tls_backend: TlsBackend,
    /// 会话缓存最大容量
    pub session_cache_max_capacity: u64,
    /// 会话缓存 TTL（秒）
    pub session_cache_ttl_secs: u64,
    /// 代理地址
    pub proxy_url: Option<String>,
    /// 代理用户名
    pub proxy_username: Option<String>,
    /// 代理密码（脱敏）
    pub proxy_password: Option<String>,
    /// 是否配置了 API Key
    pub has_api_key: bool,
    /// 是否配置了 Admin API Key
    pub has_admin_api_key: bool,
}

// ============ 批量导入凭据 ============

/// IdC 格式的凭据（从 Kiro Account Manager 导出）
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdcCredentialItem {
    /// 邮箱
    #[allow(dead_code)]
    pub email: Option<String>,
    /// 标签
    pub label: Option<String>,
    /// 访问令牌
    pub access_token: Option<String>,
    /// 刷新令牌
    pub refresh_token: Option<String>,
    /// 过期时间
    pub expires_at: Option<String>,
    /// 提供者类型
    #[allow(dead_code)]
    pub provider: Option<String>,
    /// OIDC Client ID
    pub client_id: Option<String>,
    /// OIDC Client Secret
    pub client_secret: Option<String>,
    /// Region
    pub region: Option<String>,
}

/// 批量导入凭据请求
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportCredentialsRequest {
    /// 凭据列表（IdC 格式）
    pub credentials: Vec<IdcCredentialItem>,
    /// 导入到指定池（可选，默认为 default）
    pub pool_id: Option<String>,
}

/// 批量导入凭据响应
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportCredentialsResponse {
    pub success: bool,
    pub message: String,
    /// 成功导入的数量
    pub imported_count: usize,
    /// 跳过的数量（无效凭据）
    pub skipped_count: usize,
    /// 导入的凭据 ID 列表
    pub credential_ids: Vec<u64>,
    /// 跳过的凭据信息
    pub skipped_items: Vec<String>,
}

/// 更新配置请求
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateConfigRequest {
    /// 服务器地址
    #[serde(default)]
    pub host: Option<String>,
    /// 服务器端口
    #[serde(default)]
    pub port: Option<u16>,
    /// AWS Region
    #[serde(default)]
    pub region: Option<String>,
    /// 会话缓存最大容量
    #[serde(default)]
    pub session_cache_max_capacity: Option<u64>,
    /// 会话缓存 TTL（秒）
    #[serde(default)]
    pub session_cache_ttl_secs: Option<u64>,
    /// 代理地址
    #[serde(default)]
    pub proxy_url: Option<String>,
    /// 代理用户名
    #[serde(default)]
    pub proxy_username: Option<String>,
    /// 代理密码
    #[serde(default)]
    pub proxy_password: Option<String>,
    /// API Key（用于下游客户端认证）
    #[serde(default)]
    pub api_key: Option<String>,
}

// ============ 池管理 ============

/// 池列表响应
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PoolsListResponse {
    /// 池列表
    pub pools: Vec<PoolStatusItem>,
}

/// 单个池的状态信息
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PoolStatusItem {
    /// 池 ID
    pub id: String,
    /// 池名称
    pub name: String,
    /// 描述
    pub description: Option<String>,
    /// 是否启用
    pub enabled: bool,
    /// 调度模式
    pub scheduling_mode: SchedulingMode,
    /// 是否配置了代理
    pub has_proxy: bool,
    /// 优先级
    pub priority: u32,
    /// 凭据总数
    pub total_credentials: usize,
    /// 可用凭据数量
    pub available_credentials: usize,
    /// 当前活跃凭据 ID
    pub current_id: u64,
    /// 会话缓存大小
    pub session_cache_size: u64,
    /// 轮询计数器
    pub round_robin_counter: u64,
}

/// 创建池请求
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePoolRequest {
    /// 池 ID（唯一标识）
    pub id: String,
    /// 池名称
    pub name: String,
    /// 描述
    #[serde(default)]
    pub description: Option<String>,
    /// 调度模式（默认轮询）
    #[serde(default)]
    pub scheduling_mode: SchedulingMode,
    /// 池级代理 URL
    #[serde(default)]
    pub proxy_url: Option<String>,
    /// 池级代理用户名
    #[serde(default)]
    pub proxy_username: Option<String>,
    /// 池级代理密码
    #[serde(default)]
    pub proxy_password: Option<String>,
    /// 优先级
    #[serde(default)]
    pub priority: u32,
}

/// 更新池请求
#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePoolRequest {
    /// 池名称
    #[serde(default)]
    pub name: Option<String>,
    /// 描述
    #[serde(default)]
    pub description: Option<String>,
    /// 是否启用
    #[serde(default)]
    pub enabled: Option<bool>,
    /// 调度模式
    #[serde(default)]
    pub scheduling_mode: Option<SchedulingMode>,
    /// 池级代理 URL
    #[serde(default)]
    pub proxy_url: Option<String>,
    /// 池级代理用户名
    #[serde(default)]
    pub proxy_username: Option<String>,
    /// 池级代理密码
    #[serde(default)]
    pub proxy_password: Option<String>,
    /// 优先级
    #[serde(default)]
    pub priority: Option<u32>,
}

/// 设置池禁用状态请求
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetPoolDisabledRequest {
    /// 是否禁用
    pub disabled: bool,
}

/// 分配凭据到池请求
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignCredentialToPoolRequest {
    /// 目标池 ID
    pub pool_id: String,
}

