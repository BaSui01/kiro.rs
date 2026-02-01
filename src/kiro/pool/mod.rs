//! 凭证池数据结构
//!
//! 支持将凭证分组到不同池子，每个池子独立的启用/禁用、调度模式和代理配置

mod error;

pub use error::PoolError;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::kiro::token_manager::SchedulingMode;

/// 默认池 ID
pub const DEFAULT_POOL_ID: &str = "default";

/// 凭证池配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pool {
    /// 池唯一标识（如 "default", "premium"）
    pub id: String,

    /// 池名称（用于显示）
    pub name: String,

    /// 描述（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// 是否启用
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// 调度模式
    #[serde(default)]
    pub scheduling_mode: SchedulingMode,

    /// 池级代理 URL（可选）
    /// 支持格式: http://host:port, https://host:port, socks5://host:port
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proxy_url: Option<String>,

    /// 池级代理用户名（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proxy_username: Option<String>,

    /// 池级代理密码（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proxy_password: Option<String>,

    /// 优先级（用于默认池选择，数字越小优先级越高）
    #[serde(default)]
    pub priority: u32,

    /// 创建时间
    pub created_at: DateTime<Utc>,
}

fn default_enabled() -> bool {
    true
}

impl Pool {
    /// 创建新的池配置
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: None,
            enabled: true,
            scheduling_mode: SchedulingMode::default(),
            proxy_url: None,
            proxy_username: None,
            proxy_password: None,
            priority: 0,
            created_at: Utc::now(),
        }
    }

    /// 创建默认池
    pub fn default_pool() -> Self {
        Self::new(DEFAULT_POOL_ID, "默认池")
    }

    /// 设置描述
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// 设置调度模式
    pub fn with_scheduling_mode(mut self, mode: SchedulingMode) -> Self {
        self.scheduling_mode = mode;
        self
    }

    /// 设置代理配置
    pub fn with_proxy(
        mut self,
        url: impl Into<String>,
        username: Option<String>,
        password: Option<String>,
    ) -> Self {
        self.proxy_url = Some(url.into());
        self.proxy_username = username;
        self.proxy_password = password;
        self
    }

    /// 设置优先级
    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }

    /// 检查是否配置了代理
    pub fn has_proxy(&self) -> bool {
        self.proxy_url.is_some()
    }
}

impl Default for Pool {
    fn default() -> Self {
        Self::default_pool()
    }
}

/// 池配置文件格式
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PoolsConfig {
    /// 池列表
    pub pools: Vec<Pool>,
}

impl PoolsConfig {
    /// 从文件加载池配置
    pub fn load<P: AsRef<std::path::Path>>(path: P) -> Result<Self, PoolError> {
        let path = path.as_ref();

        // 文件不存在时返回默认配置（包含默认池）
        if !path.exists() {
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(path)?;

        // 文件为空时返回默认配置
        if content.trim().is_empty() {
            return Ok(Self::default());
        }

        let config: PoolsConfig = serde_json::from_str(&content)?;
        Ok(config)
    }

    /// 保存池配置到文件
    pub fn save<P: AsRef<std::path::Path>>(&self, path: P) -> Result<(), PoolError> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// 确保默认池存在
    pub fn ensure_default_pool(&mut self) {
        if !self.pools.iter().any(|p| p.id == DEFAULT_POOL_ID) {
            self.pools.insert(0, Pool::default_pool());
        }
    }

    /// 获取池（按 ID）
    pub fn get(&self, pool_id: &str) -> Option<&Pool> {
        self.pools.iter().find(|p| p.id == pool_id)
    }

    /// 获取可变池（按 ID）
    pub fn get_mut(&mut self, pool_id: &str) -> Option<&mut Pool> {
        self.pools.iter_mut().find(|p| p.id == pool_id)
    }
}

impl Default for PoolsConfig {
    fn default() -> Self {
        Self {
            pools: vec![Pool::default_pool()],
        }
    }
}

/// 池快照（用于 Admin API）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PoolSnapshot {
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
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 凭据总数
    pub credential_count: usize,
    /// 可用凭据数量
    pub available_credential_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_new() {
        let pool = Pool::new("test", "测试池");
        assert_eq!(pool.id, "test");
        assert_eq!(pool.name, "测试池");
        assert!(pool.enabled);
        assert_eq!(pool.scheduling_mode, SchedulingMode::RoundRobin);
        assert!(pool.proxy_url.is_none());
    }

    #[test]
    fn test_pool_default() {
        let pool = Pool::default_pool();
        assert_eq!(pool.id, DEFAULT_POOL_ID);
        assert_eq!(pool.name, "默认池");
    }

    #[test]
    fn test_pool_with_proxy() {
        let pool = Pool::new("test", "测试池").with_proxy(
            "socks5://127.0.0.1:1080",
            Some("user".to_string()),
            Some("pass".to_string()),
        );
        assert_eq!(pool.proxy_url, Some("socks5://127.0.0.1:1080".to_string()));
        assert_eq!(pool.proxy_username, Some("user".to_string()));
        assert_eq!(pool.proxy_password, Some("pass".to_string()));
    }

    #[test]
    fn test_pools_config_default() {
        let config = PoolsConfig::default();
        assert_eq!(config.pools.len(), 1);
        assert_eq!(config.pools[0].id, DEFAULT_POOL_ID);
    }

    #[test]
    fn test_pool_serialization() {
        let pool = Pool::new("test", "测试池")
            .with_description("这是一个测试池")
            .with_scheduling_mode(SchedulingMode::PriorityFill);

        let json = serde_json::to_string_pretty(&pool).unwrap();
        assert!(json.contains("\"id\": \"test\""));
        assert!(json.contains("\"schedulingMode\": \"priority_fill\""));

        let parsed: Pool = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, "test");
        assert_eq!(parsed.scheduling_mode, SchedulingMode::PriorityFill);
    }
}
