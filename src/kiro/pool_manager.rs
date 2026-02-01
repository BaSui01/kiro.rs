//! 凭证池管理器
//!
//! 管理多个凭证池，每个池有独立的：
//! - 凭证列表和 Token 管理
//! - 调度模式（轮询/优先填充）
//! - 代理配置
//!
//! 支持 API Key 绑定到特定池，实现请求路由

use parking_lot::RwLock;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::http_client::ProxyConfig;
use crate::kiro::model::credentials::{CredentialsConfig, KiroCredentials};
use crate::kiro::pool::{Pool, PoolError, PoolsConfig, DEFAULT_POOL_ID};
use crate::kiro::token_manager::{MultiTokenManager, SchedulingMode};
use crate::model::config::Config;

/// 池运行时状态
pub struct PoolRuntime {
    /// 池配置
    pub config: Pool,
    /// Token 管理器
    pub token_manager: Arc<MultiTokenManager>,
    /// 池级代理配置（已解析）
    pub proxy_config: Option<ProxyConfig>,
}

impl PoolRuntime {
    /// 获取池 ID
    pub fn id(&self) -> &str {
        &self.config.id
    }

    /// 检查池是否启用
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// 获取调度模式
    pub fn scheduling_mode(&self) -> SchedulingMode {
        self.config.scheduling_mode
    }
}

/// 池管理器
///
/// 管理所有凭证池的生命周期和请求路由
pub struct PoolManager {
    /// 全局配置
    global_config: Config,
    /// 全局代理配置
    global_proxy: Option<ProxyConfig>,
    /// 池运行时映射 (pool_id -> PoolRuntime)
    pools: RwLock<HashMap<String, Arc<PoolRuntime>>>,
    /// 池配置文件路径
    pools_path: PathBuf,
    /// 凭据配置文件路径
    credentials_path: PathBuf,
}

impl PoolManager {
    /// 创建池管理器
    ///
    /// # Arguments
    /// * `global_config` - 全局配置
    /// * `global_proxy` - 全局代理配置
    /// * `pools_path` - 池配置文件路径
    /// * `credentials_path` - 凭据配置文件路径
    pub fn new(
        global_config: Config,
        global_proxy: Option<ProxyConfig>,
        pools_path: impl AsRef<Path>,
        credentials_path: impl AsRef<Path>,
    ) -> Result<Self, PoolError> {
        let pools_path = pools_path.as_ref().to_path_buf();
        let credentials_path = credentials_path.as_ref().to_path_buf();

        let manager = Self {
            global_config,
            global_proxy,
            pools: RwLock::new(HashMap::new()),
            pools_path,
            credentials_path,
        };

        // 加载池和凭据
        manager.reload()?;

        Ok(manager)
    }

    /// 重新加载池和凭据配置
    pub fn reload(&self) -> Result<(), PoolError> {
        // 加载池配置
        let mut pools_config = PoolsConfig::load(&self.pools_path).map_err(|e| {
            PoolError::ConfigLoadFailed {
                reason: format!("加载池配置失败: {}", e),
            }
        })?;

        // 确保默认池存在
        pools_config.ensure_default_pool();

        // 加载凭据配置
        let credentials_config =
            CredentialsConfig::load(&self.credentials_path).map_err(|e| {
                PoolError::ConfigLoadFailed {
                    reason: format!("加载凭据配置失败: {}", e),
                }
            })?;
        let is_multiple_format = credentials_config.is_multiple();
        let all_credentials = credentials_config.into_sorted_credentials();

        // 按 pool_id 分组凭据
        let mut credentials_by_pool: HashMap<String, Vec<KiroCredentials>> = HashMap::new();
        for cred in all_credentials {
            let pool_id = cred.pool_id.clone().unwrap_or_else(|| DEFAULT_POOL_ID.to_string());
            credentials_by_pool.entry(pool_id).or_default().push(cred);
        }

        // 为每个池创建运行时
        let mut new_pools = HashMap::new();
        for pool in pools_config.pools {
            let pool_id = pool.id.clone();
            let credentials = credentials_by_pool.remove(&pool_id).unwrap_or_default();

            // 解析池级代理配置
            let pool_proxy = self.resolve_pool_proxy(&pool);

            // 创建 Token 管理器
            let token_manager = MultiTokenManager::new(
                self.global_config.clone(),
                credentials,
                pool_proxy.clone(),
                Some(self.credentials_path.clone()),
                is_multiple_format,
            )
            .map_err(|e| PoolError::TokenManagerError(e.to_string()))?;

            // 设置调度模式
            token_manager.set_scheduling_mode(pool.scheduling_mode);

            let runtime = PoolRuntime {
                config: pool,
                token_manager: Arc::new(token_manager),
                proxy_config: pool_proxy,
            };

            new_pools.insert(pool_id, Arc::new(runtime));
        }

        // 处理没有对应池的凭据（归入默认池）
        if let Some(orphan_credentials) = credentials_by_pool.remove(DEFAULT_POOL_ID) {
            if new_pools.contains_key(DEFAULT_POOL_ID) {
                // 默认池已存在，需要合并凭据
                // 这种情况不应该发生，因为我们已经处理过了
                tracing::warn!(
                    "发现 {} 个孤儿凭据，但默认池已存在",
                    orphan_credentials.len()
                );
            }
        }

        // 更新池映射
        *self.pools.write() = new_pools;

        Ok(())
    }

    /// 解析池级代理配置
    fn resolve_pool_proxy(&self, pool: &Pool) -> Option<ProxyConfig> {
        // 池级代理优先于全局代理
        if let Some(ref proxy_url) = pool.proxy_url {
            Some(ProxyConfig {
                url: proxy_url.clone(),
                username: pool.proxy_username.clone(),
                password: pool.proxy_password.clone(),
            })
        } else {
            self.global_proxy.clone()
        }
    }

    /// 获取池（按 ID）
    pub fn get_pool(&self, pool_id: &str) -> Option<Arc<PoolRuntime>> {
        self.pools.read().get(pool_id).cloned()
    }

    /// 获取默认池
    pub fn get_default_pool(&self) -> Option<Arc<PoolRuntime>> {
        self.get_pool(DEFAULT_POOL_ID)
    }

    /// 自动路由特殊值
    pub const AUTO_ROUTE_POOL_ID: &'static str = "__auto__";

    /// 根据 API Key 绑定的 pool_id 获取池
    ///
    /// - pool_id 为 None：返回默认池
    /// - pool_id 为 "__auto__"：自动路由，按池优先级选择有可用凭据的池
    /// - pool_id 为其他值：返回指定池（如果存在且启用）
    pub fn get_pool_for_api_key(&self, pool_id: Option<&str>) -> Option<Arc<PoolRuntime>> {
        match pool_id {
            None => {
                // 未绑定池，使用默认池
                let pool = self.get_pool(DEFAULT_POOL_ID)?;
                if pool.is_enabled() {
                    Some(pool)
                } else {
                    tracing::warn!("默认池已禁用");
                    None
                }
            }
            Some(Self::AUTO_ROUTE_POOL_ID) => {
                // 自动路由：按优先级选择有可用凭据的池
                self.auto_route_pool()
            }
            Some(pool_id) => {
                // 绑定特定池
                let pool = self.get_pool(pool_id)?;
                if pool.is_enabled() {
                    Some(pool)
                } else {
                    tracing::warn!(pool_id = %pool_id, "池已禁用");
                    None
                }
            }
        }
    }

    /// 自动路由：按池优先级选择有可用凭据的池
    ///
    /// 遍历所有启用的池（按 priority 排序），返回第一个有可用凭据的池
    fn auto_route_pool(&self) -> Option<Arc<PoolRuntime>> {
        let pools = self.pools.read();

        // 收集所有启用的池并按优先级排序
        let mut enabled_pools: Vec<_> = pools
            .values()
            .filter(|p| p.is_enabled())
            .cloned()
            .collect();

        enabled_pools.sort_by_key(|p| p.config.priority);

        // 按优先级遍历，找到第一个有可用凭据的池
        for pool in enabled_pools {
            let snapshot = pool.token_manager.snapshot();
            if snapshot.available > 0 {
                tracing::debug!(
                    pool_id = %pool.config.id,
                    available = snapshot.available,
                    "自动路由选择池"
                );
                return Some(pool);
            }
        }

        tracing::warn!("自动路由：所有池都没有可用凭据");
        None
    }

    /// 获取所有池的快照
    pub fn snapshot(&self) -> Vec<PoolSnapshot> {
        self.pools
            .read()
            .values()
            .map(|runtime| {
                let snapshot = runtime.token_manager.snapshot();
                PoolSnapshot {
                    id: runtime.config.id.clone(),
                    name: runtime.config.name.clone(),
                    description: runtime.config.description.clone(),
                    enabled: runtime.config.enabled,
                    scheduling_mode: runtime.config.scheduling_mode,
                    has_proxy: runtime.config.has_proxy(),
                    priority: runtime.config.priority,
                    total_credentials: snapshot.total,
                    available_credentials: snapshot.available,
                    current_id: snapshot.current_id,
                    session_cache_size: snapshot.session_cache_size as u64,
                    round_robin_counter: snapshot.round_robin_counter,
                }
            })
            .collect()
    }

    /// 获取所有池 ID
    pub fn pool_ids(&self) -> Vec<String> {
        self.pools.read().keys().cloned().collect()
    }

    /// 获取池数量
    pub fn pool_count(&self) -> usize {
        self.pools.read().len()
    }

    // ============ 池管理 API ============

    /// 创建新池
    pub fn create_pool(&self, pool: Pool) -> Result<(), PoolError> {
        let pool_id = pool.id.clone();

        // 检查池是否已存在
        if self.pools.read().contains_key(&pool_id) {
            return Err(PoolError::PoolAlreadyExists { pool_id });
        }

        // 解析池级代理
        let pool_proxy = self.resolve_pool_proxy(&pool);

        // 创建空的 Token 管理器
        let token_manager = MultiTokenManager::new(
            self.global_config.clone(),
            vec![],
            pool_proxy.clone(),
            Some(self.credentials_path.clone()),
            true,
        )
        .map_err(|e| PoolError::TokenManagerError(e.to_string()))?;

        token_manager.set_scheduling_mode(pool.scheduling_mode);

        let runtime = PoolRuntime {
            config: pool.clone(),
            token_manager: Arc::new(token_manager),
            proxy_config: pool_proxy,
        };

        // 添加到池映射
        self.pools.write().insert(pool_id, Arc::new(runtime));

        // 持久化
        self.persist_pools()?;

        Ok(())
    }

    /// 更新池配置
    pub fn update_pool(&self, pool_id: &str, updates: UpdatePoolRequest) -> Result<(), PoolError> {
        let mut pools = self.pools.write();

        let runtime = pools.get(pool_id).ok_or_else(|| PoolError::PoolNotFound {
            pool_id: pool_id.to_string(),
        })?;

        // 创建更新后的配置
        let mut new_config = runtime.config.clone();

        if let Some(name) = updates.name {
            new_config.name = name;
        }
        if let Some(description) = updates.description {
            new_config.description = Some(description);
        }
        if let Some(enabled) = updates.enabled {
            new_config.enabled = enabled;
        }
        if let Some(scheduling_mode) = updates.scheduling_mode {
            new_config.scheduling_mode = scheduling_mode;
            runtime.token_manager.set_scheduling_mode(scheduling_mode);
        }
        if let Some(proxy_url) = updates.proxy_url {
            new_config.proxy_url = Some(proxy_url);
        }
        if let Some(proxy_username) = updates.proxy_username {
            new_config.proxy_username = Some(proxy_username);
        }
        if let Some(proxy_password) = updates.proxy_password {
            new_config.proxy_password = Some(proxy_password);
        }
        if let Some(priority) = updates.priority {
            new_config.priority = priority;
        }

        // 重新解析代理配置
        let new_proxy = self.resolve_pool_proxy(&new_config);

        // 创建新的运行时
        let new_runtime = PoolRuntime {
            config: new_config,
            token_manager: runtime.token_manager.clone(),
            proxy_config: new_proxy,
        };

        pools.insert(pool_id.to_string(), Arc::new(new_runtime));
        drop(pools);

        // 持久化
        self.persist_pools()?;

        Ok(())
    }

    /// 删除池
    pub fn delete_pool(&self, pool_id: &str) -> Result<(), PoolError> {
        if pool_id == DEFAULT_POOL_ID {
            return Err(PoolError::CannotDeleteDefaultPool);
        }

        let mut pools = self.pools.write();

        if pools.remove(pool_id).is_none() {
            return Err(PoolError::PoolNotFound {
                pool_id: pool_id.to_string(),
            });
        }

        drop(pools);

        // 持久化
        self.persist_pools()?;

        Ok(())
    }

    /// 设置池启用/禁用状态
    pub fn set_pool_disabled(&self, pool_id: &str, disabled: bool) -> Result<(), PoolError> {
        self.update_pool(
            pool_id,
            UpdatePoolRequest {
                enabled: Some(!disabled),
                ..Default::default()
            },
        )
    }

    /// 持久化池配置
    fn persist_pools(&self) -> Result<(), PoolError> {
        let pools = self.pools.read();
        let pools_config = PoolsConfig {
            pools: pools.values().map(|r| r.config.clone()).collect(),
        };
        pools_config.save(&self.pools_path)?;
        Ok(())
    }

    // ============ 凭据分配 API ============

    /// 将凭据分配到池
    ///
    /// 注意：这需要重新加载凭据配置
    pub fn assign_credential_to_pool(
        &self,
        credential_id: u64,
        pool_id: &str,
    ) -> Result<(), PoolError> {
        // 检查目标池是否存在
        if !self.pools.read().contains_key(pool_id) {
            return Err(PoolError::PoolNotFound {
                pool_id: pool_id.to_string(),
            });
        }

        // 加载凭据配置
        let content = std::fs::read_to_string(&self.credentials_path)?;
        let mut credentials: Vec<KiroCredentials> = if content.trim().is_empty() {
            vec![]
        } else {
            serde_json::from_str(&content)?
        };

        // 找到并更新凭据
        let found = credentials.iter_mut().find(|c| c.id == Some(credential_id));
        if let Some(cred) = found {
            cred.pool_id = Some(pool_id.to_string());
        } else {
            return Err(PoolError::CredentialNotFound { credential_id });
        }

        // 保存凭据配置
        let content = serde_json::to_string_pretty(&credentials)?;
        std::fs::write(&self.credentials_path, content)?;

        // 重新加载
        self.reload()?;

        Ok(())
    }
}

/// 池快照（用于 API 响应）
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PoolSnapshot {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub scheduling_mode: SchedulingMode,
    pub has_proxy: bool,
    pub priority: u32,
    pub total_credentials: usize,
    pub available_credentials: usize,
    pub current_id: u64,
    pub session_cache_size: u64,
    pub round_robin_counter: u64,
}

/// 更新池请求
#[derive(Debug, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePoolRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub enabled: Option<bool>,
    pub scheduling_mode: Option<SchedulingMode>,
    pub proxy_url: Option<String>,
    pub proxy_username: Option<String>,
    pub proxy_password: Option<String>,
    pub priority: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_pool_manager_creation() {
        let dir = tempdir().unwrap();
        let pools_path = dir.path().join("pools.json");
        let credentials_path = dir.path().join("credentials.json");

        // 创建空的凭据文件
        std::fs::write(&credentials_path, "[]").unwrap();

        let config = Config::default();
        let manager = PoolManager::new(config, None, &pools_path, &credentials_path).unwrap();

        // 应该有默认池
        assert_eq!(manager.pool_count(), 1);
        assert!(manager.get_default_pool().is_some());
    }

    #[test]
    fn test_pool_crud() {
        let dir = tempdir().unwrap();
        let pools_path = dir.path().join("pools.json");
        let credentials_path = dir.path().join("credentials.json");

        std::fs::write(&credentials_path, "[]").unwrap();

        let config = Config::default();
        let manager = PoolManager::new(config, None, &pools_path, &credentials_path).unwrap();

        // 创建新池
        let pool = Pool::new("test", "测试池");
        manager.create_pool(pool).unwrap();
        assert_eq!(manager.pool_count(), 2);

        // 获取池
        let pool = manager.get_pool("test").unwrap();
        assert_eq!(pool.config.name, "测试池");

        // 更新池
        manager
            .update_pool(
                "test",
                UpdatePoolRequest {
                    name: Some("更新后的池".to_string()),
                    ..Default::default()
                },
            )
            .unwrap();
        let pool = manager.get_pool("test").unwrap();
        assert_eq!(pool.config.name, "更新后的池");

        // 删除池
        manager.delete_pool("test").unwrap();
        assert_eq!(manager.pool_count(), 1);

        // 不能删除默认池
        assert!(manager.delete_pool(DEFAULT_POOL_ID).is_err());
    }

    #[test]
    fn test_pool_error_types() {
        let dir = tempdir().unwrap();
        let pools_path = dir.path().join("pools.json");
        let credentials_path = dir.path().join("credentials.json");

        std::fs::write(&credentials_path, "[]").unwrap();

        let config = Config::default();
        let manager = PoolManager::new(config, None, &pools_path, &credentials_path).unwrap();

        // 测试 PoolAlreadyExists
        let pool = Pool::new("default", "重复池");
        let err = manager.create_pool(pool).unwrap_err();
        assert!(err.is_pool_already_exists());

        // 测试 PoolNotFound
        let err = manager.delete_pool("nonexistent").unwrap_err();
        assert!(err.is_pool_not_found());

        // 测试 CannotDeleteDefaultPool
        let err = manager.delete_pool(DEFAULT_POOL_ID).unwrap_err();
        assert!(err.is_cannot_delete_default_pool());
    }
}
