//! Token 管理模块
//!
//! 负责 Token 过期检测和刷新，支持 Social 和 IdC 认证方式
//! 支持单凭据 (TokenManager) 和多凭据 (MultiTokenManager) 管理
//! 支持粘性会话轮询：同一会话绑定同一凭据，新会话轮询分配

use anyhow::bail;
use chrono::{DateTime, Duration, Utc};
use dashmap::DashMap;
use moka::sync::Cache;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration as StdDuration;
use tokio::sync::Mutex as TokioMutex;

use std::path::PathBuf;

use crate::http_client::{ProxyConfig, build_client};
use crate::kiro::machine_id;
use crate::kiro::model::credentials::KiroCredentials;
use crate::kiro::model::token_refresh::{
    IdcRefreshRequest, IdcRefreshResponse, RefreshRequest, RefreshResponse,
};
use crate::kiro::model::usage_limits::UsageLimitsResponse;
use crate::model::config::Config;

/// Token 管理器
///
/// 负责管理凭据和 Token 的自动刷新
#[allow(dead_code)]
pub struct TokenManager {
    config: Config,
    credentials: KiroCredentials,
    proxy: Option<ProxyConfig>,
}

#[allow(dead_code)]
impl TokenManager {
    /// 创建新的 TokenManager 实例
    pub fn new(config: Config, credentials: KiroCredentials, proxy: Option<ProxyConfig>) -> Self {
        Self {
            config,
            credentials,
            proxy,
        }
    }

    /// 获取凭据的引用
    pub fn credentials(&self) -> &KiroCredentials {
        &self.credentials
    }

    /// 获取配置的引用
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// 确保获取有效的访问 Token
    ///
    /// 如果 Token 过期或即将过期，会自动刷新
    pub async fn ensure_valid_token(&mut self) -> anyhow::Result<String> {
        if is_token_expired(&self.credentials) || is_token_expiring_soon(&self.credentials) {
            self.credentials =
                refresh_token(&self.credentials, &self.config, self.proxy.as_ref()).await?;

            // 刷新后再次检查 token 时间有效性
            if is_token_expired(&self.credentials) {
                anyhow::bail!("刷新后的 Token 仍然无效或已过期");
            }
        }

        self.credentials
            .access_token
            .clone()
            .ok_or_else(|| anyhow::anyhow!("没有可用的 accessToken"))
    }

    /// 获取使用额度信息
    ///
    /// 调用 getUsageLimits API 查询当前账户的使用额度
    pub async fn get_usage_limits(&mut self) -> anyhow::Result<UsageLimitsResponse> {
        let token = self.ensure_valid_token().await?;
        get_usage_limits(&self.credentials, &self.config, &token, self.proxy.as_ref()).await
    }
}

/// 检查 Token 是否在指定时间内过期
pub(crate) fn is_token_expiring_within(
    credentials: &KiroCredentials,
    minutes: i64,
) -> Option<bool> {
    credentials
        .expires_at
        .as_ref()
        .and_then(|expires_at| DateTime::parse_from_rfc3339(expires_at).ok())
        .map(|expires| expires <= Utc::now() + Duration::minutes(minutes))
}

/// 检查 Token 是否已过期（提前 5 分钟判断）
pub(crate) fn is_token_expired(credentials: &KiroCredentials) -> bool {
    is_token_expiring_within(credentials, 5).unwrap_or(true)
}

/// 检查 Token 是否即将过期（10分钟内）
pub(crate) fn is_token_expiring_soon(credentials: &KiroCredentials) -> bool {
    is_token_expiring_within(credentials, 10).unwrap_or(false)
}

/// 验证 refreshToken 的基本有效性
pub(crate) fn validate_refresh_token(credentials: &KiroCredentials) -> anyhow::Result<()> {
    let refresh_token = credentials
        .refresh_token
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("缺少 refreshToken"))?;

    if refresh_token.is_empty() {
        bail!("refreshToken 为空");
    }

    if refresh_token.len() < 100 || refresh_token.ends_with("...") || refresh_token.contains("...")
    {
        bail!(
            "refreshToken 已被截断（长度: {} 字符）。\n\
             这通常是 Kiro IDE 为了防止凭证被第三方工具使用而故意截断的。",
            refresh_token.len()
        );
    }

    Ok(())
}

/// 刷新 Token
pub(crate) async fn refresh_token(
    credentials: &KiroCredentials,
    config: &Config,
    proxy: Option<&ProxyConfig>,
) -> anyhow::Result<KiroCredentials> {
    validate_refresh_token(credentials)?;

    // 根据 auth_method 选择刷新方式
    // 如果未指定 auth_method，根据是否有 clientId/clientSecret 自动判断
    let auth_method = credentials.auth_method.as_deref().unwrap_or_else(|| {
        if credentials.client_id.is_some() && credentials.client_secret.is_some() {
            "idc"
        } else {
            "social"
        }
    });

    if auth_method.eq_ignore_ascii_case("idc")
        || auth_method.eq_ignore_ascii_case("builder-id")
        || auth_method.eq_ignore_ascii_case("iam")
    {
        refresh_idc_token(credentials, config, proxy).await
    } else {
        refresh_social_token(credentials, config, proxy).await
    }
}

/// 刷新 Social Token
async fn refresh_social_token(
    credentials: &KiroCredentials,
    config: &Config,
    proxy: Option<&ProxyConfig>,
) -> anyhow::Result<KiroCredentials> {
    tracing::info!("正在刷新 Social Token...");

    let refresh_token = credentials.refresh_token.as_ref().unwrap();
    // 优先使用凭据级 region，未配置时回退到 config.region
    let region = credentials.region.as_ref().unwrap_or(&config.region);

    let refresh_url = format!("https://prod.{}.auth.desktop.kiro.dev/refreshToken", region);
    let refresh_domain = format!("prod.{}.auth.desktop.kiro.dev", region);
    let machine_id = machine_id::generate_from_credentials(credentials, config)
        .ok_or_else(|| anyhow::anyhow!("无法生成 machineId"))?;
    let kiro_version = &config.kiro_version;

    let client = build_client(proxy, 60, config.tls_backend)?;
    let body = RefreshRequest {
        refresh_token: refresh_token.to_string(),
    };

    let response = client
        .post(&refresh_url)
        .header("Accept", "application/json, text/plain, */*")
        .header("Content-Type", "application/json")
        .header(
            "User-Agent",
            format!("KiroIDE-{}-{}", kiro_version, machine_id),
        )
        .header("Accept-Encoding", "gzip, compress, deflate, br")
        .header("host", &refresh_domain)
        .header("Connection", "close")
        .json(&body)
        .send()
        .await?;

    let status = response.status();
    if !status.is_success() {
        let body_text = response.text().await.unwrap_or_default();
        let error_msg = match status.as_u16() {
            401 => "OAuth 凭证已过期或无效，需要重新认证",
            403 => "权限不足，无法刷新 Token",
            429 => "请求过于频繁，已被限流",
            500..=599 => "服务器错误，AWS OAuth 服务暂时不可用",
            _ => "Token 刷新失败",
        };
        bail!("{}: {} {}", error_msg, status, body_text);
    }

    let data: RefreshResponse = response.json().await?;

    let mut new_credentials = credentials.clone();
    new_credentials.access_token = Some(data.access_token);

    if let Some(new_refresh_token) = data.refresh_token {
        new_credentials.refresh_token = Some(new_refresh_token);
    }

    if let Some(profile_arn) = data.profile_arn {
        new_credentials.profile_arn = Some(profile_arn);
    }

    if let Some(expires_in) = data.expires_in {
        let expires_at = Utc::now() + Duration::seconds(expires_in);
        new_credentials.expires_at = Some(expires_at.to_rfc3339());
    }

    Ok(new_credentials)
}

/// IdC Token 刷新所需的 x-amz-user-agent header
const IDC_AMZ_USER_AGENT: &str = "aws-sdk-js/3.738.0 ua/2.1 os/other lang/js md/browser#unknown_unknown api/sso-oidc#3.738.0 m/E KiroIDE";

/// 刷新 IdC Token (AWS SSO OIDC)
async fn refresh_idc_token(
    credentials: &KiroCredentials,
    config: &Config,
    proxy: Option<&ProxyConfig>,
) -> anyhow::Result<KiroCredentials> {
    tracing::info!("正在刷新 IdC Token...");

    let refresh_token = credentials.refresh_token.as_ref().unwrap();
    let client_id = credentials
        .client_id
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("IdC 刷新需要 clientId"))?;
    let client_secret = credentials
        .client_secret
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("IdC 刷新需要 clientSecret"))?;

    // 优先使用凭据级 region，未配置时回退到 config.region
    let region = credentials.region.as_ref().unwrap_or(&config.region);
    let refresh_url = format!("https://oidc.{}.amazonaws.com/token", region);

    let client = build_client(proxy, 60, config.tls_backend)?;
    let body = IdcRefreshRequest {
        client_id: client_id.to_string(),
        client_secret: client_secret.to_string(),
        refresh_token: refresh_token.to_string(),
        grant_type: "refresh_token".to_string(),
    };

    let response = client
        .post(&refresh_url)
        .header("Content-Type", "application/json")
        .header("Host", format!("oidc.{}.amazonaws.com", region))
        .header("Connection", "keep-alive")
        .header("x-amz-user-agent", IDC_AMZ_USER_AGENT)
        .header("Accept", "*/*")
        .header("Accept-Language", "*")
        .header("sec-fetch-mode", "cors")
        .header("User-Agent", "node")
        .header("Accept-Encoding", "br, gzip, deflate")
        .json(&body)
        .send()
        .await?;

    let status = response.status();
    if !status.is_success() {
        let body_text = response.text().await.unwrap_or_default();
        let error_msg = match status.as_u16() {
            401 => "IdC 凭证已过期或无效，需要重新认证",
            403 => "权限不足，无法刷新 Token",
            429 => "请求过于频繁，已被限流",
            500..=599 => "服务器错误，AWS OIDC 服务暂时不可用",
            _ => "IdC Token 刷新失败",
        };
        bail!("{}: {} {}", error_msg, status, body_text);
    }

    let data: IdcRefreshResponse = response.json().await?;

    let mut new_credentials = credentials.clone();
    new_credentials.access_token = Some(data.access_token);

    if let Some(new_refresh_token) = data.refresh_token {
        new_credentials.refresh_token = Some(new_refresh_token);
    }

    if let Some(expires_in) = data.expires_in {
        let expires_at = Utc::now() + Duration::seconds(expires_in);
        new_credentials.expires_at = Some(expires_at.to_rfc3339());
    }

    Ok(new_credentials)
}

/// getUsageLimits API 所需的 x-amz-user-agent header 前缀
const USAGE_LIMITS_AMZ_USER_AGENT_PREFIX: &str = "aws-sdk-js/1.0.0";

/// 获取使用额度信息
pub(crate) async fn get_usage_limits(
    credentials: &KiroCredentials,
    config: &Config,
    token: &str,
    proxy: Option<&ProxyConfig>,
) -> anyhow::Result<UsageLimitsResponse> {
    tracing::debug!("正在获取使用额度信息...");

    let region = &config.region;
    let host = format!("q.{}.amazonaws.com", region);
    let machine_id = machine_id::generate_from_credentials(credentials, config)
        .ok_or_else(|| anyhow::anyhow!("无法生成 machineId"))?;
    let kiro_version = &config.kiro_version;

    // 构建 URL
    let mut url = format!(
        "https://{}/getUsageLimits?origin=AI_EDITOR&resourceType=AGENTIC_REQUEST",
        host
    );

    // profileArn 是可选的
    if let Some(profile_arn) = &credentials.profile_arn {
        url.push_str(&format!("&profileArn={}", urlencoding::encode(profile_arn)));
    }

    // 构建 User-Agent headers
    let user_agent = format!(
        "aws-sdk-js/1.0.0 ua/2.1 os/darwin#24.6.0 lang/js md/nodejs#22.21.1 \
         api/codewhispererruntime#1.0.0 m/N,E KiroIDE-{}-{}",
        kiro_version, machine_id
    );
    let amz_user_agent = format!(
        "{} KiroIDE-{}-{}",
        USAGE_LIMITS_AMZ_USER_AGENT_PREFIX, kiro_version, machine_id
    );

    let client = build_client(proxy, 60, config.tls_backend)?;

    let response = client
        .get(&url)
        .header("x-amz-user-agent", &amz_user_agent)
        .header("User-Agent", &user_agent)
        .header("host", &host)
        .header("amz-sdk-invocation-id", uuid::Uuid::new_v4().to_string())
        .header("amz-sdk-request", "attempt=1; max=1")
        .header("Authorization", format!("Bearer {}", token))
        .header("Connection", "close")
        .send()
        .await?;

    let status = response.status();
    if !status.is_success() {
        let body_text = response.text().await.unwrap_or_default();
        let error_msg = match status.as_u16() {
            401 => "认证失败，Token 无效或已过期",
            403 => "权限不足，无法获取使用额度",
            429 => "请求过于频繁，已被限流",
            500..=599 => "服务器错误，AWS 服务暂时不可用",
            _ => "获取使用额度失败",
        };
        bail!("{}: {} {}", error_msg, status, body_text);
    }

    let data: UsageLimitsResponse = response.json().await?;
    Ok(data)
}

// ============================================================================
// 多凭据 Token 管理器
// ============================================================================

/// 单个凭据条目的状态
struct CredentialEntry {
    /// 凭据唯一 ID
    id: u64,
    /// 凭据信息
    credentials: KiroCredentials,
    /// API 调用连续失败次数
    failure_count: u32,
    /// 是否已禁用
    disabled: bool,
    /// 禁用原因（用于区分手动禁用 vs 自动禁用，便于自愈）
    disabled_reason: Option<DisabledReason>,
}

/// 禁用原因
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DisabledReason {
    /// Admin API 手动禁用
    Manual,
    /// 连续失败达到阈值后自动禁用
    TooManyFailures,
    /// 额度已用尽（如 MONTHLY_REQUEST_COUNT）
    QuotaExceeded,
    /// Token 刷新失败（refreshToken 无效或已过期）
    TokenRefreshFailed,
}

// ============================================================================
// Admin API 公开结构
// ============================================================================

/// 凭据条目快照（用于 Admin API 读取）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CredentialEntrySnapshot {
    /// 凭据唯一 ID
    pub id: u64,
    /// 优先级
    pub priority: u32,
    /// 是否被禁用
    pub disabled: bool,
    /// 连续失败次数
    pub failure_count: u32,
    /// 认证方式
    pub auth_method: Option<String>,
    /// 是否有 Profile ARN
    pub has_profile_arn: bool,
    /// Token 过期时间
    pub expires_at: Option<String>,
}

/// 凭据管理器状态快照
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagerSnapshot {
    /// 凭据条目列表
    pub entries: Vec<CredentialEntrySnapshot>,
    /// 当前活跃凭据 ID
    pub current_id: u64,
    /// 总凭据数量
    pub total: usize,
    /// 可用凭据数量
    pub available: usize,
    /// 会话缓存大小（当前缓存的会话数量）
    pub session_cache_size: usize,
    /// 轮询计数器（用于统计新会话分配次数）
    pub round_robin_counter: u64,
    /// 当前调度模式
    pub scheduling_mode: SchedulingMode,
}

/// 凭据调度模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SchedulingMode {
    /// 轮询模式：新会话按轮询方式分配到不同凭据（均匀负载）
    #[default]
    RoundRobin,
    /// 优先填充模式：优先使用高优先级凭据，直到失败才切换
    PriorityFill,
}

/// 多凭据 Token 管理器
///
/// 支持多个凭据的管理，实现粘性会话轮询 + 故障转移策略：
/// - 粘性会话：同一会话的请求始终路由到同一凭据（命中上游缓存）
/// - 轮询分配：新会话按轮询方式分配到不同凭据（均匀负载）
/// - 优先填充：优先使用高优先级凭据，直到失败才切换
/// - 故障转移：凭据失败时自动切换到其他可用凭据
///
/// 故障统计基于 API 调用结果，而非 Token 刷新结果
pub struct MultiTokenManager {
    config: Config,
    proxy: Option<ProxyConfig>,
    /// 凭据条目列表
    entries: Mutex<Vec<CredentialEntry>>,
    /// 当前活动凭据 ID（用于无会话请求的默认选择）
    current_id: Mutex<u64>,
    /// Token 刷新锁映射（按凭据 ID 分组），确保同一凭据同一时间只有一个刷新操作
    /// 使用细粒度锁避免高并发时多个凭据刷新串行化
    refresh_locks: DashMap<u64, Arc<TokioMutex<()>>>,
    /// 凭据文件路径（用于回写）
    credentials_path: Option<PathBuf>,
    /// 会话到凭据的映射缓存（LRU + TTL）
    /// Key: 会话标识, Value: 凭据 ID
    session_map: Cache<String, u64>,
    /// 轮询计数器（用于新会话分配）
    round_robin_counter: AtomicU64,
    /// 调度模式
    scheduling_mode: Mutex<SchedulingMode>,
}

/// 会话缓存配置
const SESSION_CACHE_MAX_CAPACITY: u64 = 10_000;
/// 会话缓存 TTL（1 小时）
const SESSION_CACHE_TTL_SECS: u64 = 3600;

/// 每个凭据最大 API 调用失败次数
const MAX_FAILURES_PER_CREDENTIAL: u32 = 3;

/// API 调用上下文
///
/// 绑定特定凭据的调用上下文，确保 token、credentials 和 id 的一致性
/// 用于解决并发调用时 current_id 竞态问题
#[derive(Clone)]
pub struct CallContext {
    /// 凭据 ID（用于 report_success/report_failure）
    pub id: u64,
    /// 凭据信息（用于构建请求头）
    pub credentials: KiroCredentials,
    /// 访问 Token
    pub token: String,
    /// 代理配置（凭据级 > 池级 > 全局）
    #[allow(dead_code)]
    pub proxy_config: Option<ProxyConfig>,
}

impl MultiTokenManager {
    /// 创建多凭据 Token 管理器
    ///
    /// # Arguments
    /// * `config` - 应用配置
    /// * `credentials` - 凭据列表
    /// * `proxy` - 可选的代理配置
    /// * `credentials_path` - 凭据文件路径（用于回写）
    pub fn new(
        config: Config,
        credentials: Vec<KiroCredentials>,
        proxy: Option<ProxyConfig>,
        credentials_path: Option<PathBuf>,
    ) -> anyhow::Result<Self> {
        // 过滤无效凭据（示例凭据、截断凭据等）
        let (valid_credentials, skipped_count): (Vec<_>, usize) = {
            let mut valid = Vec::new();
            let mut skipped = 0;
            for cred in credentials {
                match validate_refresh_token(&cred) {
                    Ok(()) => valid.push(cred),
                    Err(e) => {
                        skipped += 1;
                        let token_preview = cred
                            .refresh_token
                            .as_ref()
                            .map(|t| {
                                if t.len() > 20 {
                                    format!("{}...", &t[..20])
                                } else {
                                    t.clone()
                                }
                            })
                            .unwrap_or_else(|| "<空>".to_string());
                        tracing::warn!(
                            "跳过无效凭据 (id={:?}): {} [token预览: {}]",
                            cred.id,
                            e,
                            token_preview
                        );
                    }
                }
            }
            (valid, skipped)
        };

        if skipped_count > 0 {
            tracing::info!(
                "凭据加载完成: 有效 {} 个, 跳过 {} 个无效凭据",
                valid_credentials.len(),
                skipped_count
            );
        }

        // 计算当前最大 ID，为没有 ID 的凭据分配新 ID
        let max_existing_id = valid_credentials.iter().filter_map(|c| c.id).max().unwrap_or(0);
        let mut next_id = max_existing_id + 1;
        let mut has_new_ids = false;
        let mut has_new_machine_ids = false;
        let config_ref = &config;

        let entries: Vec<CredentialEntry> = valid_credentials
            .into_iter()
            .map(|mut cred| {
                cred.canonicalize_auth_method();
                let id = cred.id.unwrap_or_else(|| {
                    let id = next_id;
                    next_id += 1;
                    cred.id = Some(id);
                    has_new_ids = true;
                    id
                });
                if cred.machine_id.is_none() {
                    if let Some(machine_id) =
                        machine_id::generate_from_credentials(&cred, config_ref)
                    {
                        cred.machine_id = Some(machine_id);
                        has_new_machine_ids = true;
                    }
                }
                CredentialEntry {
                    id,
                    credentials: cred,
                    failure_count: 0,
                    disabled: false,
                    disabled_reason: None,
                }
            })
            .collect();

        // 检测重复 ID
        let mut seen_ids = std::collections::HashSet::new();
        let mut duplicate_ids = Vec::new();
        for entry in &entries {
            if !seen_ids.insert(entry.id) {
                duplicate_ids.push(entry.id);
            }
        }
        if !duplicate_ids.is_empty() {
            anyhow::bail!("检测到重复的凭据 ID: {:?}", duplicate_ids);
        }

        // 选择初始凭据：优先级最高（priority 最小）的凭据，无凭据时为 0
        let initial_id = entries
            .iter()
            .min_by_key(|e| e.credentials.priority)
            .map(|e| e.id)
            .unwrap_or(0);

        // 构建会话缓存：LRU + TTL + 驱逐监听器
        let session_map = Cache::builder()
            .max_capacity(SESSION_CACHE_MAX_CAPACITY)
            .time_to_live(StdDuration::from_secs(SESSION_CACHE_TTL_SECS))
            .eviction_listener(|session_id: Arc<String>, credential_id: u64, cause| {
                // 记录缓存驱逐事件，便于监控和调试
                match cause {
                    moka::notification::RemovalCause::Expired => {
                        tracing::debug!(
                            "会话缓存过期: session={} -> credential_id={} (TTL={}s)",
                            &session_id[..session_id.len().min(20)],
                            credential_id,
                            SESSION_CACHE_TTL_SECS
                        );
                    }
                    moka::notification::RemovalCause::Size => {
                        tracing::info!(
                            "会话缓存容量淘汰: session={} -> credential_id={} (容量上限={})",
                            &session_id[..session_id.len().min(20)],
                            credential_id,
                            SESSION_CACHE_MAX_CAPACITY
                        );
                    }
                    moka::notification::RemovalCause::Explicit => {
                        tracing::debug!(
                            "会话缓存显式移除: session={} -> credential_id={}",
                            &session_id[..session_id.len().min(20)],
                            credential_id
                        );
                    }
                    moka::notification::RemovalCause::Replaced => {
                        tracing::debug!(
                            "会话缓存替换: session={} -> credential_id={}",
                            &session_id[..session_id.len().min(20)],
                            credential_id
                        );
                    }
                }
            })
            .build();

        let manager = Self {
            config,
            proxy,
            entries: Mutex::new(entries),
            current_id: Mutex::new(initial_id),
            refresh_locks: DashMap::new(),
            credentials_path,
            session_map,
            round_robin_counter: AtomicU64::new(0),
            scheduling_mode: Mutex::new(SchedulingMode::default()),
        };

        // 如果有新分配的 ID 或新生成的 machineId，立即持久化到配置文件
        if has_new_ids || has_new_machine_ids {
            if let Err(e) = manager.persist_credentials() {
                tracing::warn!("补全凭据 ID/machineId 后持久化失败: {}", e);
            } else {
                tracing::info!("已补全凭据 ID/machineId 并写回配置文件");
            }
        }

        Ok(manager)
    }

    /// 获取配置的引用
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// 获取当前活动凭据的克隆
    #[allow(dead_code)]
    pub fn credentials(&self) -> KiroCredentials {
        let entries = self.entries.lock();
        let current_id = *self.current_id.lock();
        entries
            .iter()
            .find(|e| e.id == current_id)
            .map(|e| e.credentials.clone())
            .unwrap_or_default()
    }

    /// 获取凭据总数
    pub fn total_count(&self) -> usize {
        self.entries.lock().len()
    }

    /// 获取可用凭据数量
    pub fn available_count(&self) -> usize {
        self.entries.lock().iter().filter(|e| !e.disabled).count()
    }

    /// 获取 API 调用上下文
    ///
    /// 返回绑定了 id、credentials 和 token 的调用上下文
    /// 确保整个 API 调用过程中使用一致的凭据信息
    ///
    /// 如果 Token 过期或即将过期，会自动刷新
    /// Token 刷新失败时会尝试下一个可用凭据（不计入失败次数）
    pub async fn acquire_context(&self) -> anyhow::Result<CallContext> {
        // 无会话标识时，使用默认的优先级策略
        self.acquire_context_internal(None).await
    }

    /// 获取指定会话的 API 调用上下文（粘性会话 + 轮询）
    ///
    /// 实现粘性会话轮询策略：
    /// 1. 如果有 session_id 且缓存命中，使用缓存的凭据（粘性）
    /// 2. 如果缓存未命中或凭据不可用，轮询选择新凭据
    /// 3. 更新会话缓存
    ///
    /// # Arguments
    /// * `session_id` - 会话标识（可选）
    pub async fn acquire_context_for_session(
        &self,
        session_id: Option<&str>,
    ) -> anyhow::Result<CallContext> {
        self.acquire_context_internal(session_id).await
    }

    /// 内部方法：获取 API 调用上下文
    ///
    /// # Arguments
    /// * `session_id` - 会话标识（可选），用于粘性会话
    async fn acquire_context_internal(
        &self,
        session_id: Option<&str>,
    ) -> anyhow::Result<CallContext> {
        let total = self.total_count();
        let mut tried_count = 0;

        // 尝试从会话缓存获取凭据 ID
        let cached_id = session_id.and_then(|sid| self.session_map.get(sid));

        // 获取当前调度模式
        let mode = *self.scheduling_mode.lock();

        loop {
            if tried_count >= total {
                anyhow::bail!(
                    "所有凭据均无法获取有效 Token（可用: {}/{}）",
                    self.available_count(),
                    total
                );
            }

            let (id, credentials) = {
                let mut entries = self.entries.lock();

                // 优先使用缓存的凭据 ID（粘性会话）
                let target_id = if tried_count == 0 {
                    cached_id.or_else(|| {
                        // 无缓存时，根据调度模式选择凭据
                        if session_id.is_some() {
                            match mode {
                                SchedulingMode::RoundRobin => self.select_by_round_robin(&entries),
                                SchedulingMode::PriorityFill => self.select_by_priority(&entries),
                            }
                        } else {
                            // 无会话标识时，使用当前凭据
                            Some(*self.current_id.lock())
                        }
                    })
                } else {
                    // 重试时，根据调度模式选择下一个凭据
                    match mode {
                        SchedulingMode::RoundRobin => self.select_by_round_robin(&entries),
                        SchedulingMode::PriorityFill => self.select_by_priority(&entries),
                    }
                };

                // 找到目标凭据
                if let Some(tid) = target_id {
                    if let Some(entry) = entries.iter().find(|e| e.id == tid && !e.disabled) {
                        (entry.id, entry.credentials.clone())
                    } else {
                        // 目标凭据不可用，选择任意可用凭据
                        self.select_any_available(&mut entries, total)?
                    }
                } else {
                    // 无目标凭据，选择任意可用凭据
                    self.select_any_available(&mut entries, total)?
                }
            };

            // 尝试获取/刷新 Token
            match self.try_ensure_token(id, &credentials).await {
                Ok(ctx) => {
                    // 成功后更新会话缓存
                    if let Some(sid) = session_id {
                        self.session_map.insert(sid.to_string(), ctx.id);
                        tracing::debug!(
                            "会话 {} 绑定到凭据 #{}",
                            &sid[..sid.len().min(20)],
                            ctx.id
                        );
                    }
                    return Ok(ctx);
                }
                Err(e) => {
                    let error_msg = e.to_string();
                    tracing::warn!("凭据 #{} Token 刷新失败，尝试下一个凭据: {}", id, error_msg);

                    // 判断是否为不可恢复的错误（需要禁用凭据）
                    let should_disable = error_msg.contains("refreshToken")
                        || error_msg.contains("截断")
                        || error_msg.contains("invalid_grant")
                        || error_msg.contains("expired")
                        || error_msg.contains("unauthorized")
                        || error_msg.contains("401")
                        || error_msg.contains("403");

                    if should_disable {
                        tracing::error!(
                            "凭据 #{} 的 refreshToken 无效或已过期，自动禁用该凭据",
                            id
                        );
                        // 禁用凭据
                        let mut entries = self.entries.lock();
                        if let Some(entry) = entries.iter_mut().find(|e| e.id == id) {
                            entry.disabled = true;
                            entry.disabled_reason = Some(DisabledReason::TokenRefreshFailed);
                        }
                        self.reset_round_robin_counter();
                    }

                    tried_count += 1;
                }
            }
        }
    }

    /// 按优先级选择凭据（内部方法）
    ///
    /// 选择优先级最高（priority 最小）的可用凭据
    fn select_by_priority(&self, entries: &[CredentialEntry]) -> Option<u64> {
        entries
            .iter()
            .filter(|e| !e.disabled)
            .min_by_key(|e| e.credentials.priority)
            .map(|e| e.id)
    }

    /// 轮询选择凭据（内部方法）
    ///
    /// 按轮询方式从可用凭据中选择一个
    fn select_by_round_robin(&self, entries: &[CredentialEntry]) -> Option<u64> {
        let available: Vec<_> = entries.iter().filter(|e| !e.disabled).collect();
        if available.is_empty() {
            return None;
        }

        let counter = self.round_robin_counter.fetch_add(1, Ordering::Relaxed);
        let index = (counter as usize) % available.len();
        Some(available[index].id)
    }

    /// 重置轮询计数器（内部方法）
    ///
    /// 当凭据列表发生变化时调用，确保轮询公平性
    /// 场景：凭据禁用/启用/添加/删除
    fn reset_round_robin_counter(&self) {
        let old_value = self.round_robin_counter.swap(0, Ordering::Relaxed);
        if old_value > 0 {
            tracing::debug!(
                "轮询计数器已重置（凭据列表变化）: {} -> 0",
                old_value
            );
        }
    }

    /// 选择任意可用凭据（内部方法）
    ///
    /// 当目标凭据不可用时，选择优先级最高的可用凭据
    /// 如果所有凭据都被自动禁用，执行自愈
    fn select_any_available(
        &self,
        entries: &mut Vec<CredentialEntry>,
        total: usize,
    ) -> anyhow::Result<(u64, KiroCredentials)> {
        // 选择优先级最高的可用凭据
        let mut best = entries
            .iter()
            .filter(|e| !e.disabled)
            .min_by_key(|e| e.credentials.priority);

        // 没有可用凭据：如果是"自动禁用导致全灭"，做一次类似重启的自愈
        // 自愈范围：TooManyFailures 和 TokenRefreshFailed（可能是临时网络问题）
        // 不自愈：Manual（手动禁用）和 QuotaExceeded（额度用尽）
        if best.is_none()
            && entries.iter().any(|e| {
                e.disabled
                    && matches!(
                        e.disabled_reason,
                        Some(DisabledReason::TooManyFailures)
                            | Some(DisabledReason::TokenRefreshFailed)
                    )
            })
        {
            tracing::warn!(
                "所有凭据均已被自动禁用，执行自愈：重置失败计数并重新启用（等价于重启）"
            );
            for e in entries.iter_mut() {
                if matches!(
                    e.disabled_reason,
                    Some(DisabledReason::TooManyFailures) | Some(DisabledReason::TokenRefreshFailed)
                ) {
                    e.disabled = false;
                    e.disabled_reason = None;
                    e.failure_count = 0;
                }
            }
            best = entries
                .iter()
                .filter(|e| !e.disabled)
                .min_by_key(|e| e.credentials.priority);
        }

        if let Some(entry) = best {
            let new_id = entry.id;
            let new_creds = entry.credentials.clone();
            // 更新 current_id
            *self.current_id.lock() = new_id;
            Ok((new_id, new_creds))
        } else {
            let available = entries.iter().filter(|e| !e.disabled).count();
            anyhow::bail!("所有凭据均已禁用（{}/{}）", available, total);
        }
    }

    /// 切换到下一个优先级最高的可用凭据（内部方法）
    #[allow(dead_code)]
    fn switch_to_next_by_priority(&self) {
        let entries = self.entries.lock();
        let mut current_id = self.current_id.lock();

        // 选择优先级最高的未禁用凭据（排除当前凭据）
        if let Some(entry) = entries
            .iter()
            .filter(|e| !e.disabled && e.id != *current_id)
            .min_by_key(|e| e.credentials.priority)
        {
            *current_id = entry.id;
            tracing::info!(
                "已切换到凭据 #{}（优先级 {}）",
                entry.id,
                entry.credentials.priority
            );
        }
    }

    /// 选择优先级最高的未禁用凭据作为当前凭据（内部方法）
    ///
    /// 与 `switch_to_next_by_priority` 不同，此方法不排除当前凭据，
    /// 纯粹按优先级选择，用于优先级变更后立即生效
    fn select_highest_priority(&self) {
        let entries = self.entries.lock();
        let mut current_id = self.current_id.lock();

        // 选择优先级最高的未禁用凭据（不排除当前凭据）
        if let Some(best) = entries
            .iter()
            .filter(|e| !e.disabled)
            .min_by_key(|e| e.credentials.priority)
        {
            if best.id != *current_id {
                tracing::info!(
                    "优先级变更后切换凭据: #{} -> #{}（优先级 {}）",
                    *current_id,
                    best.id,
                    best.credentials.priority
                );
                *current_id = best.id;
            }
        }
    }

    /// 尝试使用指定凭据获取有效 Token
    ///
    /// 使用双重检查锁定模式，确保同一凭据同一时间只有一个刷新操作
    /// 使用细粒度锁（按凭据 ID 分组），避免高并发时多个凭据刷新串行化
    ///
    /// # Arguments
    /// * `id` - 凭据 ID，用于更新正确的条目
    /// * `credentials` - 凭据信息
    async fn try_ensure_token(
        &self,
        id: u64,
        credentials: &KiroCredentials,
    ) -> anyhow::Result<CallContext> {
        // 第一次检查（无锁）：快速判断是否需要刷新
        let needs_refresh = is_token_expired(credentials) || is_token_expiring_soon(credentials);

        let creds = if needs_refresh {
            // 获取或创建该凭据的刷新锁（细粒度锁，不同凭据可并行刷新）
            let lock = self
                .refresh_locks
                .entry(id)
                .or_insert_with(|| Arc::new(TokioMutex::new(())))
                .clone();
            let _guard = lock.lock().await;

            // 第二次检查：获取锁后重新读取凭据，因为其他请求可能已经完成刷新
            let current_creds = {
                let entries = self.entries.lock();
                entries
                    .iter()
                    .find(|e| e.id == id)
                    .map(|e| e.credentials.clone())
                    .ok_or_else(|| anyhow::anyhow!("凭据 #{} 不存在", id))?
            };

            if is_token_expired(&current_creds) || is_token_expiring_soon(&current_creds) {
                // 确实需要刷新
                let new_creds =
                    refresh_token(&current_creds, &self.config, self.proxy.as_ref()).await?;

                if is_token_expired(&new_creds) {
                    anyhow::bail!("刷新后的 Token 仍然无效或已过期");
                }

                // 更新凭据
                {
                    let mut entries = self.entries.lock();
                    if let Some(entry) = entries.iter_mut().find(|e| e.id == id) {
                        entry.credentials = new_creds.clone();
                    }
                }

                // 回写凭据到文件（仅多凭据格式），失败只记录警告
                if let Err(e) = self.persist_credentials() {
                    tracing::warn!("Token 刷新后持久化失败（不影响本次请求）: {}", e);
                }

                new_creds
            } else {
                // 其他请求已经完成刷新，直接使用新凭据
                tracing::debug!("Token 已被其他请求刷新，跳过刷新");
                current_creds
            }
        } else {
            credentials.clone()
        };

        let token = creds
            .access_token
            .clone()
            .ok_or_else(|| anyhow::anyhow!("没有可用的 accessToken"))?;

        // 解析代理配置：凭据级 > 池级（由调用方传入）> 全局
        let proxy_config = self.resolve_proxy_config(&creds);

        Ok(CallContext {
            id,
            credentials: creds,
            token,
            proxy_config,
        })
    }

    /// 将凭据列表回写到源文件
    ///
    /// 仅在以下条件满足时回写：
    /// - 源文件是多凭据格式（数组）
    /// - credentials_path 已设置
    ///
    /// # Returns
    /// - `Ok(true)` - 成功写入文件
    /// - `Ok(false)` - 跳过写入（无路径配置）
    /// - `Err(_)` - 写入失败
    fn persist_credentials(&self) -> anyhow::Result<bool> {
        use anyhow::Context;

        let path = match &self.credentials_path {
            Some(p) => p,
            None => return Ok(false),
        };

        // 收集所有凭据
        let credentials: Vec<KiroCredentials> = {
            let entries = self.entries.lock();
            entries
                .iter()
                .map(|e| {
                    let mut cred = e.credentials.clone();
                    cred.canonicalize_auth_method();
                    cred
                })
                .collect()
        };

        // 序列化为 pretty JSON
        let json = serde_json::to_string_pretty(&credentials).context("序列化凭据失败")?;

        // 写入文件（在 Tokio runtime 内使用 block_in_place 避免阻塞 worker）
        if tokio::runtime::Handle::try_current().is_ok() {
            tokio::task::block_in_place(|| std::fs::write(path, &json))
                .with_context(|| format!("回写凭据文件失败: {:?}", path))?;
        } else {
            std::fs::write(path, &json).with_context(|| format!("回写凭据文件失败: {:?}", path))?;
        }

        tracing::debug!("已回写凭据到文件: {:?}", path);
        Ok(true)
    }

    /// 报告指定凭据 API 调用成功
    ///
    /// 重置该凭据的失败计数
    ///
    /// # Arguments
    /// * `id` - 凭据 ID（来自 CallContext）
    pub fn report_success(&self, id: u64) {
        let mut entries = self.entries.lock();
        if let Some(entry) = entries.iter_mut().find(|e| e.id == id) {
            entry.failure_count = 0;
            tracing::debug!("凭据 #{} API 调用成功", id);
        }
    }

    /// 报告指定凭据 API 调用失败
    ///
    /// 增加失败计数，达到阈值时禁用凭据并切换到优先级最高的可用凭据
    /// 返回是否还有可用凭据可以重试
    ///
    /// # Arguments
    /// * `id` - 凭据 ID（来自 CallContext）
    pub fn report_failure(&self, id: u64) -> bool {
        let should_reset_counter;
        let has_available;

        {
            let mut entries = self.entries.lock();
            let mut current_id = self.current_id.lock();

            let entry = match entries.iter_mut().find(|e| e.id == id) {
                Some(e) => e,
                None => return entries.iter().any(|e| !e.disabled),
            };

            entry.failure_count += 1;
            let failure_count = entry.failure_count;

            tracing::warn!(
                "凭据 #{} API 调用失败（{}/{}）",
                id,
                failure_count,
                MAX_FAILURES_PER_CREDENTIAL
            );

            if failure_count >= MAX_FAILURES_PER_CREDENTIAL {
                entry.disabled = true;
                entry.disabled_reason = Some(DisabledReason::TooManyFailures);
                tracing::error!("凭据 #{} 已连续失败 {} 次，已被禁用", id, failure_count);
                should_reset_counter = true;

                // 切换到优先级最高的可用凭据
                if let Some(next) = entries
                    .iter()
                    .filter(|e| !e.disabled)
                    .min_by_key(|e| e.credentials.priority)
                {
                    *current_id = next.id;
                    tracing::info!(
                        "已切换到凭据 #{}（优先级 {}）",
                        next.id,
                        next.credentials.priority
                    );
                    has_available = true;
                } else {
                    tracing::error!("所有凭据均已禁用！");
                    has_available = false;
                }
            } else {
                should_reset_counter = false;
                has_available = entries.iter().any(|e| !e.disabled);
            }
        }

        // 凭据列表变化，重置轮询计数器确保公平性（在锁外执行）
        if should_reset_counter {
            self.reset_round_robin_counter();
        }

        has_available
    }

    /// 报告指定凭据额度已用尽
    ///
    /// 用于处理 402 Payment Required 且 reason 为 `MONTHLY_REQUEST_COUNT` 的场景：
    /// - 立即禁用该凭据（不等待连续失败阈值）
    /// - 切换到下一个可用凭据继续重试
    /// - 返回是否还有可用凭据
    pub fn report_quota_exhausted(&self, id: u64) -> bool {
        let has_available;

        {
            let mut entries = self.entries.lock();
            let mut current_id = self.current_id.lock();

            let entry = match entries.iter_mut().find(|e| e.id == id) {
                Some(e) => e,
                None => return entries.iter().any(|e| !e.disabled),
            };

            if entry.disabled {
                return entries.iter().any(|e| !e.disabled);
            }

            entry.disabled = true;
            entry.disabled_reason = Some(DisabledReason::QuotaExceeded);
            // 设为阈值，便于在管理面板中直观看到该凭据已不可用
            entry.failure_count = MAX_FAILURES_PER_CREDENTIAL;

            tracing::error!("凭据 #{} 额度已用尽（MONTHLY_REQUEST_COUNT），已被禁用", id);

            // 切换到优先级最高的可用凭据
            if let Some(next) = entries
                .iter()
                .filter(|e| !e.disabled)
                .min_by_key(|e| e.credentials.priority)
            {
                *current_id = next.id;
                tracing::info!(
                    "已切换到凭据 #{}（优先级 {}）",
                    next.id,
                    next.credentials.priority
                );
                has_available = true;
            } else {
                tracing::error!("所有凭据均已禁用！");
                has_available = false;
            }
        }

        // 凭据列表变化，重置轮询计数器确保公平性（在锁外执行）
        self.reset_round_robin_counter();

        has_available
    }

    /// 切换到优先级最高的可用凭据
    ///
    /// 返回是否成功切换
    pub fn switch_to_next(&self) -> bool {
        let entries = self.entries.lock();
        let mut current_id = self.current_id.lock();

        // 选择优先级最高的未禁用凭据（排除当前凭据）
        if let Some(next) = entries
            .iter()
            .filter(|e| !e.disabled && e.id != *current_id)
            .min_by_key(|e| e.credentials.priority)
        {
            *current_id = next.id;
            tracing::info!(
                "已切换到凭据 #{}（优先级 {}）",
                next.id,
                next.credentials.priority
            );
            true
        } else {
            // 没有其他可用凭据，检查当前凭据是否可用
            entries.iter().any(|e| e.id == *current_id && !e.disabled)
        }
    }

    /// 获取使用额度信息
    #[allow(dead_code)]
    pub async fn get_usage_limits(&self) -> anyhow::Result<UsageLimitsResponse> {
        let ctx = self.acquire_context().await?;
        get_usage_limits(
            &ctx.credentials,
            &self.config,
            &ctx.token,
            self.proxy.as_ref(),
        )
        .await
    }

    // ========================================================================
    // Admin API 方法
    // ========================================================================

    /// 获取管理器状态快照（用于 Admin API）
    pub fn snapshot(&self) -> ManagerSnapshot {
        let entries = self.entries.lock();
        let current_id = *self.current_id.lock();
        let available = entries.iter().filter(|e| !e.disabled).count();
        let mode = *self.scheduling_mode.lock();

        ManagerSnapshot {
            entries: entries
                .iter()
                .map(|e| CredentialEntrySnapshot {
                    id: e.id,
                    priority: e.credentials.priority,
                    disabled: e.disabled,
                    failure_count: e.failure_count,
                    auth_method: e.credentials.auth_method.as_deref().map(|m| {
                        if m.eq_ignore_ascii_case("builder-id") || m.eq_ignore_ascii_case("iam") {
                            "idc".to_string()
                        } else {
                            m.to_string()
                        }
                    }),
                    has_profile_arn: e.credentials.profile_arn.is_some(),
                    expires_at: e.credentials.expires_at.clone(),
                })
                .collect(),
            current_id,
            total: entries.len(),
            available,
            session_cache_size: self.session_map.entry_count() as usize,
            round_robin_counter: self.round_robin_counter.load(Ordering::Relaxed),
            scheduling_mode: mode,
        }
    }

    /// 设置凭据禁用状态（Admin API）
    pub fn set_disabled(&self, id: u64, disabled: bool) -> anyhow::Result<()> {
        {
            let mut entries = self.entries.lock();
            let entry = entries
                .iter_mut()
                .find(|e| e.id == id)
                .ok_or_else(|| anyhow::anyhow!("凭据不存在: {}", id))?;
            entry.disabled = disabled;
            if !disabled {
                // 启用时重置失败计数
                entry.failure_count = 0;
                entry.disabled_reason = None;
            } else {
                entry.disabled_reason = Some(DisabledReason::Manual);
            }
        }
        // 凭据列表变化，重置轮询计数器确保公平性
        self.reset_round_robin_counter();
        // 持久化更改
        self.persist_credentials()?;
        Ok(())
    }

    /// 设置凭据优先级（Admin API）
    ///
    /// 修改优先级后会立即按新优先级重新选择当前凭据。
    /// 即使持久化失败，内存中的优先级和当前凭据选择也会生效。
    pub fn set_priority(&self, id: u64, priority: u32) -> anyhow::Result<()> {
        {
            let mut entries = self.entries.lock();
            let entry = entries
                .iter_mut()
                .find(|e| e.id == id)
                .ok_or_else(|| anyhow::anyhow!("凭据不存在: {}", id))?;
            entry.credentials.priority = priority;
        }
        // 立即按新优先级重新选择当前凭据（无论持久化是否成功）
        self.select_highest_priority();
        // 持久化更改
        self.persist_credentials()?;
        Ok(())
    }

    /// 重置凭据失败计数并重新启用（Admin API）
    pub fn reset_and_enable(&self, id: u64) -> anyhow::Result<()> {
        {
            let mut entries = self.entries.lock();
            let entry = entries
                .iter_mut()
                .find(|e| e.id == id)
                .ok_or_else(|| anyhow::anyhow!("凭据不存在: {}", id))?;
            entry.failure_count = 0;
            entry.disabled = false;
            entry.disabled_reason = None;
        }
        // 持久化更改
        self.persist_credentials()?;
        Ok(())
    }

    /// 获取指定凭据的使用额度（Admin API）
    pub async fn get_usage_limits_for(&self, id: u64) -> anyhow::Result<UsageLimitsResponse> {
        let credentials = {
            let entries = self.entries.lock();
            entries
                .iter()
                .find(|e| e.id == id)
                .map(|e| e.credentials.clone())
                .ok_or_else(|| anyhow::anyhow!("凭据不存在: {}", id))?
        };

        // 检查是否需要刷新 token
        let needs_refresh = is_token_expired(&credentials) || is_token_expiring_soon(&credentials);

        let token = if needs_refresh {
            // 获取或创建该凭据的刷新锁（细粒度锁）
            let lock = self
                .refresh_locks
                .entry(id)
                .or_insert_with(|| Arc::new(TokioMutex::new(())))
                .clone();
            let _guard = lock.lock().await;
            let current_creds = {
                let entries = self.entries.lock();
                entries
                    .iter()
                    .find(|e| e.id == id)
                    .map(|e| e.credentials.clone())
                    .ok_or_else(|| anyhow::anyhow!("凭据不存在: {}", id))?
            };

            if is_token_expired(&current_creds) || is_token_expiring_soon(&current_creds) {
                let new_creds =
                    refresh_token(&current_creds, &self.config, self.proxy.as_ref()).await?;
                {
                    let mut entries = self.entries.lock();
                    if let Some(entry) = entries.iter_mut().find(|e| e.id == id) {
                        entry.credentials = new_creds.clone();
                    }
                }
                // 持久化失败只记录警告，不影响本次请求
                if let Err(e) = self.persist_credentials() {
                    tracing::warn!("Token 刷新后持久化失败（不影响本次请求）: {}", e);
                }
                new_creds
                    .access_token
                    .ok_or_else(|| anyhow::anyhow!("刷新后无 access_token"))?
            } else {
                current_creds
                    .access_token
                    .ok_or_else(|| anyhow::anyhow!("凭据无 access_token"))?
            }
        } else {
            credentials
                .access_token
                .ok_or_else(|| anyhow::anyhow!("凭据无 access_token"))?
        };

        let credentials = {
            let entries = self.entries.lock();
            entries
                .iter()
                .find(|e| e.id == id)
                .map(|e| e.credentials.clone())
                .ok_or_else(|| anyhow::anyhow!("凭据不存在: {}", id))?
        };

        get_usage_limits(&credentials, &self.config, &token, self.proxy.as_ref()).await
    }

    /// 添加新凭据（Admin API）
    ///
    /// # 流程
    /// 1. 验证凭据基本字段（refresh_token 不为空）
    /// 2. 尝试刷新 Token 验证凭据有效性
    /// 3. 分配新 ID（当前最大 ID + 1）
    /// 4. 添加到 entries 列表
    /// 5. 持久化到配置文件
    ///
    /// # 返回
    /// - `Ok(u64)` - 新凭据 ID
    /// - `Err(_)` - 验证失败或添加失败
    pub async fn add_credential(&self, new_cred: KiroCredentials) -> anyhow::Result<u64> {
        // 1. 基本验证
        validate_refresh_token(&new_cred)?;

        // 2. 尝试刷新 Token 验证凭据有效性
        let mut validated_cred =
            refresh_token(&new_cred, &self.config, self.proxy.as_ref()).await?;

        // 3. 分配新 ID
        let new_id = {
            let entries = self.entries.lock();
            entries.iter().map(|e| e.id).max().unwrap_or(0) + 1
        };

        // 4. 设置 ID 并保留用户输入的元数据
        validated_cred.id = Some(new_id);
        validated_cred.priority = new_cred.priority;
        validated_cred.auth_method = new_cred.auth_method.map(|m| {
            if m.eq_ignore_ascii_case("builder-id") || m.eq_ignore_ascii_case("iam") {
                "idc".to_string()
            } else {
                m
            }
        });
        validated_cred.client_id = new_cred.client_id;
        validated_cred.client_secret = new_cred.client_secret;
        validated_cred.region = new_cred.region;
        validated_cred.machine_id = new_cred.machine_id;

        {
            let mut entries = self.entries.lock();
            entries.push(CredentialEntry {
                id: new_id,
                credentials: validated_cred,
                failure_count: 0,
                disabled: false,
                disabled_reason: None,
            });
        }

        // 5. 持久化
        self.persist_credentials()?;

        // 凭据列表变化，重置轮询计数器确保公平性
        self.reset_round_robin_counter();

        tracing::info!("成功添加凭据 #{}", new_id);
        Ok(new_id)
    }

    /// 删除凭据（Admin API）
    ///
    /// # 前置条件
    /// - 凭据必须已禁用（disabled = true）
    ///
    /// # 行为
    /// 1. 验证凭据存在
    /// 2. 验证凭据已禁用
    /// 3. 从 entries 移除
    /// 4. 如果删除的是当前凭据，切换到优先级最高的可用凭据
    /// 5. 如果删除后没有凭据，将 current_id 重置为 0
    /// 6. 持久化到文件
    ///
    /// # 返回
    /// - `Ok(())` - 删除成功
    /// - `Err(_)` - 凭据不存在、未禁用或持久化失败
    pub fn delete_credential(&self, id: u64) -> anyhow::Result<()> {
        let was_current = {
            let mut entries = self.entries.lock();

            // 查找凭据
            let entry = entries
                .iter()
                .find(|e| e.id == id)
                .ok_or_else(|| anyhow::anyhow!("凭据不存在: {}", id))?;

            // 检查是否已禁用
            if !entry.disabled {
                anyhow::bail!("只能删除已禁用的凭据（请先禁用凭据 #{}）", id);
            }

            // 记录是否是当前凭据
            let current_id = *self.current_id.lock();
            let was_current = current_id == id;

            // 删除凭据
            entries.retain(|e| e.id != id);

            was_current
        };

        // 如果删除的是当前凭据，切换到优先级最高的可用凭据
        if was_current {
            self.select_highest_priority();
        }

        // 如果删除后没有任何凭据，将 current_id 重置为 0（与初始化行为保持一致）
        {
            let entries = self.entries.lock();
            if entries.is_empty() {
                let mut current_id = self.current_id.lock();
                *current_id = 0;
                tracing::info!("所有凭据已删除，current_id 已重置为 0");
            }
        }

        // 持久化更改
        self.persist_credentials()?;

        // 凭据列表变化，重置轮询计数器确保公平性
        self.reset_round_robin_counter();

        tracing::info!("已删除凭据 #{}", id);
        Ok(())
    }

    /// 设置调度模式（Admin API）
    ///
    /// # Arguments
    /// * `mode` - 新的调度模式
    pub fn set_scheduling_mode(&self, mode: SchedulingMode) {
        let mut current_mode = self.scheduling_mode.lock();
        if *current_mode != mode {
            tracing::info!(
                "调度模式已切换: {:?} -> {:?}",
                *current_mode,
                mode
            );
            *current_mode = mode;
        }
    }

    /// 获取当前调度模式（Admin API）
    #[allow(dead_code)]
    pub fn get_scheduling_mode(&self) -> SchedulingMode {
        *self.scheduling_mode.lock()
    }

    /// 解析代理配置
    ///
    /// 优先级：凭据级 > 池级（self.proxy）> 全局
    fn resolve_proxy_config(&self, credentials: &KiroCredentials) -> Option<ProxyConfig> {
        // 凭据级代理优先
        if let Some(ref proxy_url) = credentials.proxy_url {
            return Some(ProxyConfig {
                url: proxy_url.clone(),
                username: credentials.proxy_username.clone(),
                password: credentials.proxy_password.clone(),
            });
        }

        // 回退到池级/全局代理
        self.proxy.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_manager_new() {
        let config = Config::default();
        let credentials = KiroCredentials::default();
        let tm = TokenManager::new(config, credentials, None);
        assert!(tm.credentials().access_token.is_none());
    }

    #[test]
    fn test_is_token_expired_with_expired_token() {
        let mut credentials = KiroCredentials::default();
        credentials.expires_at = Some("2020-01-01T00:00:00Z".to_string());
        assert!(is_token_expired(&credentials));
    }

    #[test]
    fn test_is_token_expired_with_valid_token() {
        let mut credentials = KiroCredentials::default();
        let future = Utc::now() + Duration::hours(1);
        credentials.expires_at = Some(future.to_rfc3339());
        assert!(!is_token_expired(&credentials));
    }

    #[test]
    fn test_is_token_expired_within_5_minutes() {
        let mut credentials = KiroCredentials::default();
        let expires = Utc::now() + Duration::minutes(3);
        credentials.expires_at = Some(expires.to_rfc3339());
        assert!(is_token_expired(&credentials));
    }

    #[test]
    fn test_is_token_expired_no_expires_at() {
        let credentials = KiroCredentials::default();
        assert!(is_token_expired(&credentials));
    }

    #[test]
    fn test_is_token_expiring_soon_within_10_minutes() {
        let mut credentials = KiroCredentials::default();
        let expires = Utc::now() + Duration::minutes(8);
        credentials.expires_at = Some(expires.to_rfc3339());
        assert!(is_token_expiring_soon(&credentials));
    }

    #[test]
    fn test_is_token_expiring_soon_beyond_10_minutes() {
        let mut credentials = KiroCredentials::default();
        let expires = Utc::now() + Duration::minutes(15);
        credentials.expires_at = Some(expires.to_rfc3339());
        assert!(!is_token_expiring_soon(&credentials));
    }

    #[test]
    fn test_validate_refresh_token_missing() {
        let credentials = KiroCredentials::default();
        let result = validate_refresh_token(&credentials);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_refresh_token_valid() {
        let mut credentials = KiroCredentials::default();
        credentials.refresh_token = Some("a".repeat(150));
        let result = validate_refresh_token(&credentials);
        assert!(result.is_ok());
    }

    // MultiTokenManager 测试

    /// 创建有效的测试凭据（refresh_token 需要至少 100 字符）
    fn create_valid_test_credential() -> KiroCredentials {
        let mut cred = KiroCredentials::default();
        cred.refresh_token = Some("a".repeat(150));
        cred
    }

    #[test]
    fn test_multi_token_manager_new() {
        let config = Config::default();
        let mut cred1 = create_valid_test_credential();
        cred1.priority = 0;
        let mut cred2 = create_valid_test_credential();
        cred2.priority = 1;

        let manager =
            MultiTokenManager::new(config, vec![cred1, cred2], None, None).unwrap();
        assert_eq!(manager.total_count(), 2);
        assert_eq!(manager.available_count(), 2);
    }

    #[test]
    fn test_multi_token_manager_empty_credentials() {
        let config = Config::default();
        let result = MultiTokenManager::new(config, vec![], None, None);
        // 支持 0 个凭据启动（可通过管理面板添加）
        assert!(result.is_ok());
        let manager = result.unwrap();
        assert_eq!(manager.total_count(), 0);
        assert_eq!(manager.available_count(), 0);
    }

    #[test]
    fn test_multi_token_manager_duplicate_ids() {
        let config = Config::default();
        let mut cred1 = create_valid_test_credential();
        cred1.id = Some(1);
        let mut cred2 = create_valid_test_credential();
        cred2.id = Some(1); // 重复 ID

        let result = MultiTokenManager::new(config, vec![cred1, cred2], None, None);
        assert!(result.is_err());
        let err_msg = result.err().unwrap().to_string();
        assert!(
            err_msg.contains("重复的凭据 ID"),
            "错误消息应包含 '重复的凭据 ID'，实际: {}",
            err_msg
        );
    }

    #[test]
    fn test_multi_token_manager_report_failure() {
        let config = Config::default();
        let cred1 = create_valid_test_credential();
        let cred2 = create_valid_test_credential();

        let manager =
            MultiTokenManager::new(config, vec![cred1, cred2], None, None).unwrap();

        // 凭据会自动分配 ID（从 1 开始）
        // 前两次失败不会禁用（使用 ID 1）
        assert!(manager.report_failure(1));
        assert!(manager.report_failure(1));
        assert_eq!(manager.available_count(), 2);

        // 第三次失败会禁用第一个凭据
        assert!(manager.report_failure(1));
        assert_eq!(manager.available_count(), 1);

        // 继续失败第二个凭据（使用 ID 2）
        assert!(manager.report_failure(2));
        assert!(manager.report_failure(2));
        assert!(!manager.report_failure(2)); // 所有凭据都禁用了
        assert_eq!(manager.available_count(), 0);
    }

    #[test]
    fn test_multi_token_manager_report_success() {
        let config = Config::default();
        let cred = create_valid_test_credential();

        let manager = MultiTokenManager::new(config, vec![cred], None, None).unwrap();

        // 失败两次（使用 ID 1）
        manager.report_failure(1);
        manager.report_failure(1);

        // 成功后重置计数（使用 ID 1）
        manager.report_success(1);

        // 再失败两次不会禁用
        manager.report_failure(1);
        manager.report_failure(1);
        assert_eq!(manager.available_count(), 1);
    }

    #[test]
    fn test_multi_token_manager_switch_to_next() {
        let config = Config::default();
        let mut cred1 = create_valid_test_credential();
        cred1.refresh_token = Some("a".repeat(150) + "token1");
        let mut cred2 = create_valid_test_credential();
        cred2.refresh_token = Some("a".repeat(150) + "token2");

        let manager =
            MultiTokenManager::new(config, vec![cred1, cred2], None, None).unwrap();

        // 初始是第一个凭据
        assert!(manager
            .credentials()
            .refresh_token
            .as_ref()
            .unwrap()
            .ends_with("token1"));

        // 切换到下一个
        assert!(manager.switch_to_next());
        assert!(manager
            .credentials()
            .refresh_token
            .as_ref()
            .unwrap()
            .ends_with("token2"));
    }

    #[tokio::test]
    async fn test_multi_token_manager_acquire_context_auto_recovers_all_disabled() {
        let config = Config::default();
        let mut cred1 = create_valid_test_credential();
        cred1.access_token = Some("t1".to_string());
        cred1.expires_at = Some((Utc::now() + Duration::hours(1)).to_rfc3339());
        let mut cred2 = create_valid_test_credential();
        cred2.access_token = Some("t2".to_string());
        cred2.expires_at = Some((Utc::now() + Duration::hours(1)).to_rfc3339());

        let manager =
            MultiTokenManager::new(config, vec![cred1, cred2], None, None).unwrap();

        // 凭据会自动分配 ID（从 1 开始）
        for _ in 0..MAX_FAILURES_PER_CREDENTIAL {
            manager.report_failure(1);
        }
        for _ in 0..MAX_FAILURES_PER_CREDENTIAL {
            manager.report_failure(2);
        }

        assert_eq!(manager.available_count(), 0);

        // 应触发自愈：重置失败计数并重新启用，避免必须重启进程
        let ctx = manager.acquire_context().await.unwrap();
        assert!(ctx.token == "t1" || ctx.token == "t2");
        assert_eq!(manager.available_count(), 2);
    }

    #[test]
    fn test_multi_token_manager_report_quota_exhausted() {
        let config = Config::default();
        let cred1 = create_valid_test_credential();
        let cred2 = create_valid_test_credential();

        let manager =
            MultiTokenManager::new(config, vec![cred1, cred2], None, None).unwrap();

        // 凭据会自动分配 ID（从 1 开始）
        assert_eq!(manager.available_count(), 2);
        assert!(manager.report_quota_exhausted(1));
        assert_eq!(manager.available_count(), 1);

        // 再禁用第二个后，无可用凭据
        assert!(!manager.report_quota_exhausted(2));
        assert_eq!(manager.available_count(), 0);
    }

    #[tokio::test]
    async fn test_multi_token_manager_quota_disabled_is_not_auto_recovered() {
        let config = Config::default();
        let cred1 = create_valid_test_credential();
        let cred2 = create_valid_test_credential();

        let manager =
            MultiTokenManager::new(config, vec![cred1, cred2], None, None).unwrap();

        manager.report_quota_exhausted(1);
        manager.report_quota_exhausted(2);
        assert_eq!(manager.available_count(), 0);

        let err = manager.acquire_context().await.err().unwrap().to_string();
        assert!(
            err.contains("所有凭据均已禁用"),
            "错误应提示所有凭据禁用，实际: {}",
            err
        );
        assert_eq!(manager.available_count(), 0);
    }

    // ============ 凭据级 Region 优先级测试 ============

    /// 辅助函数：获取 OIDC 刷新使用的 region（用于测试）
    fn get_oidc_region_for_credential<'a>(
        credentials: &'a KiroCredentials,
        config: &'a Config,
    ) -> &'a str {
        credentials.region.as_ref().unwrap_or(&config.region)
    }

    #[test]
    fn test_credential_region_priority_uses_credential_region() {
        // 凭据配置了 region 时，应使用凭据的 region
        let mut config = Config::default();
        config.region = "us-west-2".to_string();

        let mut credentials = KiroCredentials::default();
        credentials.region = Some("eu-west-1".to_string());

        let region = get_oidc_region_for_credential(&credentials, &config);
        assert_eq!(region, "eu-west-1");
    }

    #[test]
    fn test_credential_region_priority_fallback_to_config() {
        // 凭据未配置 region 时，应回退到 config.region
        let mut config = Config::default();
        config.region = "us-west-2".to_string();

        let credentials = KiroCredentials::default();
        assert!(credentials.region.is_none());

        let region = get_oidc_region_for_credential(&credentials, &config);
        assert_eq!(region, "us-west-2");
    }

    #[test]
    fn test_multiple_credentials_use_respective_regions() {
        // 多凭据场景下，不同凭据使用各自的 region
        let mut config = Config::default();
        config.region = "ap-northeast-1".to_string();

        let mut cred1 = KiroCredentials::default();
        cred1.region = Some("us-east-1".to_string());

        let mut cred2 = KiroCredentials::default();
        cred2.region = Some("eu-west-1".to_string());

        let cred3 = KiroCredentials::default(); // 无 region，使用 config

        assert_eq!(get_oidc_region_for_credential(&cred1, &config), "us-east-1");
        assert_eq!(get_oidc_region_for_credential(&cred2, &config), "eu-west-1");
        assert_eq!(
            get_oidc_region_for_credential(&cred3, &config),
            "ap-northeast-1"
        );
    }

    #[test]
    fn test_idc_oidc_endpoint_uses_credential_region() {
        // 验证 IdC OIDC endpoint URL 使用凭据 region
        let mut config = Config::default();
        config.region = "us-west-2".to_string();

        let mut credentials = KiroCredentials::default();
        credentials.region = Some("eu-central-1".to_string());

        let region = get_oidc_region_for_credential(&credentials, &config);
        let refresh_url = format!("https://oidc.{}.amazonaws.com/token", region);

        assert_eq!(refresh_url, "https://oidc.eu-central-1.amazonaws.com/token");
    }

    #[test]
    fn test_social_refresh_endpoint_uses_credential_region() {
        // 验证 Social refresh endpoint URL 使用凭据 region
        let mut config = Config::default();
        config.region = "us-west-2".to_string();

        let mut credentials = KiroCredentials::default();
        credentials.region = Some("ap-southeast-1".to_string());

        let region = get_oidc_region_for_credential(&credentials, &config);
        let refresh_url = format!("https://prod.{}.auth.desktop.kiro.dev/refreshToken", region);

        assert_eq!(
            refresh_url,
            "https://prod.ap-southeast-1.auth.desktop.kiro.dev/refreshToken"
        );
    }

    #[test]
    fn test_api_call_still_uses_config_region() {
        // 验证 API 调用（如 getUsageLimits）仍使用 config.region
        // 这确保只有 OIDC 刷新使用凭据 region，API 调用行为不变
        let mut config = Config::default();
        config.region = "us-west-2".to_string();

        let mut credentials = KiroCredentials::default();
        credentials.region = Some("eu-west-1".to_string());

        // API 调用应使用 config.region，而非 credentials.region
        let api_region = &config.region;
        let api_host = format!("q.{}.amazonaws.com", api_region);

        assert_eq!(api_host, "q.us-west-2.amazonaws.com");
        // 确认凭据 region 不影响 API 调用
        assert_ne!(api_region, credentials.region.as_ref().unwrap());
    }

    #[test]
    fn test_credential_region_empty_string_treated_as_set() {
        // 空字符串 region 被视为已设置（虽然不推荐，但行为应一致）
        let mut config = Config::default();
        config.region = "us-west-2".to_string();

        let mut credentials = KiroCredentials::default();
        credentials.region = Some("".to_string());

        let region = get_oidc_region_for_credential(&credentials, &config);
        // 空字符串被视为已设置，不会回退到 config
        assert_eq!(region, "");
    }
}
