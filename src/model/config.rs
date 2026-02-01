use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum TlsBackend {
    Rustls,
    NativeTls,
}

impl Default for TlsBackend {
    fn default() -> Self {
        Self::Rustls
    }
}

/// KNA 应用配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    #[serde(default = "default_host")]
    pub host: String,

    #[serde(default = "default_port")]
    pub port: u16,

    #[serde(default = "default_region")]
    pub region: String,

    #[serde(default = "default_kiro_version")]
    pub kiro_version: String,

    #[serde(default)]
    pub machine_id: Option<String>,

    #[serde(default)]
    pub api_key: Option<String>,

    #[serde(default = "default_system_version")]
    pub system_version: String,

    #[serde(default = "default_node_version")]
    pub node_version: String,

    #[serde(default = "default_tls_backend")]
    pub tls_backend: TlsBackend,

    /// 外部 count_tokens API 地址（可选）
    #[serde(default)]
    pub count_tokens_api_url: Option<String>,

    /// count_tokens API 密钥（可选）
    #[serde(default)]
    pub count_tokens_api_key: Option<String>,

    /// count_tokens API 认证类型（可选，"x-api-key" 或 "bearer"，默认 "x-api-key"）
    #[serde(default = "default_count_tokens_auth_type")]
    pub count_tokens_auth_type: String,

    /// HTTP 代理地址（可选）
    /// 支持格式: http://host:port, https://host:port, socks5://host:port
    #[serde(default)]
    pub proxy_url: Option<String>,

    /// 代理认证用户名（可选）
    #[serde(default)]
    pub proxy_username: Option<String>,

    /// 代理认证密码（可选）
    #[serde(default)]
    pub proxy_password: Option<String>,

    /// Admin API 密钥（可选，启用 Admin API 功能）
    #[serde(default)]
    pub admin_api_key: Option<String>,

    /// 会话缓存最大容量（默认 10000）
    #[serde(default = "default_session_cache_max_capacity")]
    pub session_cache_max_capacity: u64,

    /// 会话缓存 TTL（秒，默认 3600 = 1 小时）
    #[serde(default = "default_session_cache_ttl_secs")]
    pub session_cache_ttl_secs: u64,
}

fn default_host() -> String {
    "127.0.0.1".to_string()
}

fn default_port() -> u16 {
    8080
}

fn default_region() -> String {
    "us-east-1".to_string()
}

fn default_kiro_version() -> String {
    "0.8.0".to_string()
}

fn default_system_version() -> String {
    const SYSTEM_VERSIONS: &[&str] = &["darwin#24.6.0", "win32#10.0.22631"];
    SYSTEM_VERSIONS[fastrand::usize(..SYSTEM_VERSIONS.len())].to_string()
}

fn default_node_version() -> String {
    "22.21.1".to_string()
}

fn default_count_tokens_auth_type() -> String {
    "x-api-key".to_string()
}

fn default_tls_backend() -> TlsBackend {
    TlsBackend::Rustls
}

fn default_session_cache_max_capacity() -> u64 {
    10_000
}

fn default_session_cache_ttl_secs() -> u64 {
    3600
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            region: default_region(),
            kiro_version: default_kiro_version(),
            machine_id: None,
            api_key: None,
            system_version: default_system_version(),
            node_version: default_node_version(),
            tls_backend: default_tls_backend(),
            count_tokens_api_url: None,
            count_tokens_api_key: None,
            count_tokens_auth_type: default_count_tokens_auth_type(),
            proxy_url: None,
            proxy_username: None,
            proxy_password: None,
            admin_api_key: None,
            session_cache_max_capacity: default_session_cache_max_capacity(),
            session_cache_ttl_secs: default_session_cache_ttl_secs(),
        }
    }
}

impl Config {
    /// 获取默认配置文件路径
    pub fn default_config_path() -> &'static str {
        "config.json"
    }

    /// 从文件加载配置
    pub fn load<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let path = path.as_ref();
        if !path.exists() {
            // 配置文件不存在，返回默认配置
            return Ok(Self::default());
        }

        let content = fs::read_to_string(path)?;
        let config: Config = serde_json::from_str(&content)?;
        Ok(config)
    }

    /// 保存配置到文件
    pub fn save<P: AsRef<Path>>(&self, path: P) -> anyhow::Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    /// 验证配置有效性
    ///
    /// 检查必填字段和格式是否正确
    /// 注意：apiKey 不是启动必须的，可以后续通过前端配置
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        // 检查 host
        if self.host.trim().is_empty() {
            errors.push("host 不能为空".to_string());
        }

        // 检查 port
        if self.port == 0 {
            errors.push("port 不能为 0".to_string());
        }

        // 检查 region
        if self.region.trim().is_empty() {
            errors.push("region 不能为空".to_string());
        }

        // apiKey 不是启动必须的，可以后续通过前端配置
        // 只在配置了但为空时警告
        if self.api_key.as_ref().is_some_and(|k| k.trim().is_empty()) {
            errors.push("apiKey 配置为空字符串，请移除或填写有效值".to_string());
        }

        // 检查代理 URL 格式
        if let Some(ref proxy_url) = self.proxy_url {
            if !proxy_url.is_empty()
                && !proxy_url.starts_with("http://")
                && !proxy_url.starts_with("https://")
                && !proxy_url.starts_with("socks5://")
            {
                errors.push(format!(
                    "proxyUrl 格式不正确: {}，应以 http://、https:// 或 socks5:// 开头",
                    proxy_url
                ));
            }
        }

        // 检查缓存配置
        if self.session_cache_max_capacity == 0 {
            errors.push("sessionCacheMaxCapacity 不能为 0".to_string());
        }

        if self.session_cache_ttl_secs == 0 {
            errors.push("sessionCacheTtlSecs 不能为 0".to_string());
        }

        // 检查 count_tokens_auth_type
        let valid_auth_types = ["x-api-key", "bearer"];
        if !valid_auth_types.contains(&self.count_tokens_auth_type.as_str()) {
            errors.push(format!(
                "countTokensAuthType 无效: {}，应为 'x-api-key' 或 'bearer'",
                self.count_tokens_auth_type
            ));
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}
