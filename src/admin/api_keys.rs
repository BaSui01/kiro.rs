//! API Key 管理模块
//!
//! 支持多 API Key 的 CRUD 操作，持久化到 api_keys.json

use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// API Key 操作错误
#[derive(Debug, Error)]
pub enum ApiKeyError {
    #[error("API Key 不存在: {0}")]
    NotFound(u64),

    #[error("API Key 名称已存在: {0}")]
    DuplicateName(String),

    #[error("保存失败: {0}")]
    PersistError(#[from] std::io::Error),

    #[error("序列化失败: {0}")]
    SerializeError(#[from] serde_json::Error),
}

/// API Key 条目
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiKey {
    /// 唯一 ID
    pub id: u64,
    /// 名称
    pub name: String,
    /// API Key 值
    pub key: String,
    /// 描述
    #[serde(default)]
    pub description: Option<String>,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 是否启用
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// 绑定的池 ID（未配置时使用默认池）
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pool_id: Option<String>,
}

fn default_enabled() -> bool {
    true
}

/// API Key 脱敏显示
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiKeyMasked {
    pub id: u64,
    pub name: String,
    /// 脱敏后的 Key（只显示前 8 位）
    pub key: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub enabled: bool,
    /// 绑定的池 ID
    pub pool_id: Option<String>,
}

impl From<&ApiKey> for ApiKeyMasked {
    fn from(key: &ApiKey) -> Self {
        let masked_key = if key.key.len() > 8 {
            format!("{}***", &key.key[..8])
        } else {
            "***".to_string()
        };

        Self {
            id: key.id,
            name: key.name.clone(),
            key: masked_key,
            description: key.description.clone(),
            created_at: key.created_at,
            enabled: key.enabled,
            pool_id: key.pool_id.clone(),
        }
    }
}

/// 创建 API Key 请求
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateApiKeyRequest {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    /// 可选，如果不提供则自动生成
    #[serde(default)]
    pub key: Option<String>,
    /// 绑定的池 ID
    #[serde(default)]
    pub pool_id: Option<String>,
}

/// 更新 API Key 请求
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateApiKeyRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub enabled: Option<bool>,
    /// 绑定的池 ID
    /// - 不传此字段：不修改
    /// - 传 null：解绑（清除 pool_id）
    /// - 传字符串：绑定到指定池
    #[serde(default, deserialize_with = "deserialize_optional_nullable")]
    pub pool_id: Option<Option<String>>,
}

/// 自定义反序列化器，用于区分 "字段不存在" 和 "字段为 null"
/// - 字段不存在 -> None（不修改）
/// - 字段为 null -> Some(None)（清除）
/// - 字段有值 -> Some(Some(value))（设置）
fn deserialize_optional_nullable<'de, D>(deserializer: D) -> Result<Option<Option<String>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    // 如果字段存在，反序列化为 Option<String>
    // null -> Some(None), "value" -> Some(Some("value"))
    let value: Option<String> = Option::deserialize(deserializer)?;
    Ok(Some(value))
}

/// API Key 管理器
pub struct ApiKeyManager {
    keys: RwLock<Vec<ApiKey>>,
    file_path: PathBuf,
    next_id: RwLock<u64>,
}

impl ApiKeyManager {
    /// 创建新的 API Key 管理器
    pub fn new<P: AsRef<Path>>(file_path: P) -> anyhow::Result<Self> {
        let file_path = file_path.as_ref().to_path_buf();
        let keys = Self::load_from_file(&file_path)?;

        // 计算下一个 ID
        let max_id = keys.iter().map(|k| k.id).max().unwrap_or(0);

        Ok(Self {
            keys: RwLock::new(keys),
            file_path,
            next_id: RwLock::new(max_id + 1),
        })
    }

    /// 从文件加载 API Keys
    fn load_from_file(path: &Path) -> anyhow::Result<Vec<ApiKey>> {
        if !path.exists() {
            return Ok(Vec::new());
        }

        let content = fs::read_to_string(path)?;
        if content.trim().is_empty() {
            return Ok(Vec::new());
        }

        let keys: Vec<ApiKey> = serde_json::from_str(&content)?;
        Ok(keys)
    }

    /// 保存到文件
    fn persist(&self) -> Result<(), ApiKeyError> {
        let keys = self.keys.read();
        let content = serde_json::to_string_pretty(&*keys)?;
        fs::write(&self.file_path, content)?;
        Ok(())
    }

    /// 获取所有 API Keys（脱敏）
    pub fn list(&self) -> Vec<ApiKeyMasked> {
        self.keys.read().iter().map(ApiKeyMasked::from).collect()
    }

    /// 验证 API Key 是否有效
    #[allow(dead_code)]
    pub fn validate(&self, key: &str) -> bool {
        self.keys
            .read()
            .iter()
            .any(|k| k.enabled && k.key == key)
    }

    /// 验证 API Key 并返回绑定的 pool_id
    ///
    /// 返回 Some(pool_id) 如果 Key 有效，pool_id 可能为 None（使用默认池）
    /// 返回 None 如果 Key 无效或被禁用
    pub fn validate_and_get_pool(&self, key: &str) -> Option<Option<String>> {
        self.keys
            .read()
            .iter()
            .find(|k| k.enabled && k.key == key)
            .map(|k| k.pool_id.clone())
    }

    /// 创建新的 API Key
    #[allow(dead_code)]
    pub fn create(&self, req: CreateApiKeyRequest) -> Result<ApiKeyMasked, ApiKeyError> {
        // 检查名称唯一性
        {
            let keys = self.keys.read();
            if keys.iter().any(|k| k.name == req.name) {
                return Err(ApiKeyError::DuplicateName(req.name));
            }
        }

        let key_value = req.key.unwrap_or_else(|| Self::generate_key());

        let id = {
            let mut next_id = self.next_id.write();
            let id = *next_id;
            *next_id += 1;
            id
        };

        let api_key = ApiKey {
            id,
            name: req.name,
            key: key_value,
            description: req.description,
            created_at: Utc::now(),
            enabled: true,
            pool_id: req.pool_id,
        };

        let masked = ApiKeyMasked::from(&api_key);

        {
            let mut keys = self.keys.write();
            keys.push(api_key);
        }

        self.persist()?;
        Ok(masked)
    }

    /// 创建新的 API Key（返回完整 Key，仅在创建时使用）
    pub fn create_with_full_key(&self, req: CreateApiKeyRequest) -> Result<ApiKey, ApiKeyError> {
        // 检查名称唯一性
        {
            let keys = self.keys.read();
            if keys.iter().any(|k| k.name == req.name) {
                return Err(ApiKeyError::DuplicateName(req.name));
            }
        }

        let key_value = req.key.unwrap_or_else(|| Self::generate_key());

        let id = {
            let mut next_id = self.next_id.write();
            let id = *next_id;
            *next_id += 1;
            id
        };

        let api_key = ApiKey {
            id,
            name: req.name,
            key: key_value,
            description: req.description,
            created_at: Utc::now(),
            enabled: true,
            pool_id: req.pool_id,
        };

        let result = api_key.clone();

        {
            let mut keys = self.keys.write();
            keys.push(api_key);
        }

        self.persist()?;
        Ok(result)
    }

    /// 更新 API Key
    pub fn update(&self, id: u64, req: UpdateApiKeyRequest) -> Result<ApiKeyMasked, ApiKeyError> {
        let mut keys = self.keys.write();

        let key = keys
            .iter_mut()
            .find(|k| k.id == id)
            .ok_or(ApiKeyError::NotFound(id))?;

        if let Some(name) = req.name {
            key.name = name;
        }
        if let Some(description) = req.description {
            key.description = Some(description);
        }
        if let Some(enabled) = req.enabled {
            key.enabled = enabled;
        }
        // pool_id 处理：
        // - None: 不修改
        // - Some(None): 解绑（清除 pool_id）
        // - Some(Some(value)): 绑定到指定池
        if let Some(pool_id_option) = req.pool_id {
            key.pool_id = pool_id_option;
        }

        let masked = ApiKeyMasked::from(&*key);
        drop(keys);

        self.persist()?;
        Ok(masked)
    }

    /// 删除 API Key
    pub fn delete(&self, id: u64) -> Result<(), ApiKeyError> {
        let mut keys = self.keys.write();

        let pos = keys
            .iter()
            .position(|k| k.id == id)
            .ok_or(ApiKeyError::NotFound(id))?;

        keys.remove(pos);
        drop(keys);

        self.persist()?;
        Ok(())
    }

    /// 生成随机 API Key（使用密码学安全随机数）
    fn generate_key() -> String {
        use rand::distributions::Alphanumeric;
        use rand::{rngs::OsRng, Rng};

        let mut key = String::with_capacity(40);
        key.push_str("sk-");

        // 使用 OsRng 生成密码学安全的随机字符
        let chars: String = OsRng
            .sample_iter(&Alphanumeric)
            .take(32)
            .map(char::from)
            .collect();

        key.push_str(&chars);
        key
    }

    /// 获取 API Key 数量
    #[allow(dead_code)]
    pub fn count(&self) -> usize {
        self.keys.read().len()
    }

    /// 获取启用的 API Key 数量
    #[allow(dead_code)]
    pub fn enabled_count(&self) -> usize {
        self.keys.read().iter().filter(|k| k.enabled).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_generate_key() {
        let key = ApiKeyManager::generate_key();
        assert!(key.starts_with("sk-"));
        assert_eq!(key.len(), 35); // "sk-" + 32 chars
    }

    #[test]
    fn test_api_key_crud() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("api_keys.json");

        let manager = ApiKeyManager::new(&file_path).unwrap();

        // Create
        let key = manager
            .create_with_full_key(CreateApiKeyRequest {
                name: "Test Key".to_string(),
                description: Some("Test description".to_string()),
                key: None,
                pool_id: None,
            })
            .unwrap();

        assert_eq!(key.name, "Test Key");
        assert!(key.key.starts_with("sk-"));

        // List
        let keys = manager.list();
        assert_eq!(keys.len(), 1);
        assert!(keys[0].key.ends_with("***"));

        // Validate
        assert!(manager.validate(&key.key));
        assert!(!manager.validate("invalid-key"));

        // Update
        let updated = manager
            .update(
                key.id,
                UpdateApiKeyRequest {
                    name: Some("Updated Key".to_string()),
                    description: None,
                    enabled: Some(false),
                    pool_id: None, // 不修改 pool_id
                },
            )
            .unwrap();

        assert_eq!(updated.name, "Updated Key");
        assert!(!updated.enabled);

        // Validate disabled key
        assert!(!manager.validate(&key.key));

        // Delete
        manager.delete(key.id).unwrap();
        assert_eq!(manager.count(), 0);
    }

    #[test]
    fn test_api_key_with_pool_id() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("api_keys.json");

        let manager = ApiKeyManager::new(&file_path).unwrap();

        // Create with pool_id
        let key = manager
            .create_with_full_key(CreateApiKeyRequest {
                name: "Premium Key".to_string(),
                description: None,
                key: None,
                pool_id: Some("premium".to_string()),
            })
            .unwrap();

        assert_eq!(key.pool_id, Some("premium".to_string()));

        // Validate and get pool
        let pool_id = manager.validate_and_get_pool(&key.key);
        assert_eq!(pool_id, Some(Some("premium".to_string())));

        // Update pool_id
        let updated = manager
            .update(
                key.id,
                UpdateApiKeyRequest {
                    name: None,
                    description: None,
                    enabled: None,
                    pool_id: Some(Some("default".to_string())), // 绑定到 default 池
                },
            )
            .unwrap();

        assert_eq!(updated.pool_id, Some("default".to_string()));

        // Unbind pool_id (set to null)
        let unbound = manager
            .update(
                key.id,
                UpdateApiKeyRequest {
                    name: None,
                    description: None,
                    enabled: None,
                    pool_id: Some(None), // 解绑
                },
            )
            .unwrap();

        assert_eq!(unbound.pool_id, None);
    }
}
