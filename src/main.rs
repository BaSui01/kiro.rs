mod admin;
mod admin_ui;
mod anthropic;
mod common;
mod http_client;
mod kiro;
mod model;
pub mod token;

use std::sync::Arc;

use clap::Parser;
use kiro::model::credentials::{CredentialsConfig, KiroCredentials};
use kiro::pool_manager::PoolManager;
use kiro::provider::KiroProvider;
use kiro::token_manager::MultiTokenManager;
use model::arg::Args;
use model::config::Config;

#[tokio::main]
async fn main() {
    // 解析命令行参数
    let args = Args::parse();

    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // 确保 config 目录存在
    if let Err(e) = std::fs::create_dir_all("config") {
        tracing::warn!("创建 config 目录失败: {}", e);
    }

    // 加载配置
    let config_path = args
        .config
        .unwrap_or_else(|| Config::default_config_path().to_string());
    let config = Config::load(&config_path).unwrap_or_else(|e| {
        tracing::error!("加载配置失败: {}", e);
        std::process::exit(1);
    });

    // 验证配置
    if let Err(errors) = config.validate() {
        tracing::error!("配置验证失败:");
        for error in &errors {
            tracing::error!("  - {}", error);
        }
        std::process::exit(1);
    }

    // 加载凭证（仅支持数组格式，文件不存在时使用空列表）
    let credentials_path = args
        .credentials
        .unwrap_or_else(|| KiroCredentials::default_credentials_path().to_string());
    let credentials_list = match CredentialsConfig::load(&credentials_path) {
        Ok(credentials_config) => credentials_config.into_sorted_credentials(),
        Err(e) => {
            // 凭证文件不存在或解析失败，使用空列表（可以后续通过前端添加）
            tracing::warn!("加载凭证失败: {}，将以空凭证启动", e);
            tracing::warn!("可以通过 Admin UI 添加凭证");
            Vec::new()
        }
    };

    tracing::info!("已加载 {} 个凭据配置", credentials_list.len());

    // 获取第一个凭据用于日志显示
    let first_credentials = credentials_list.first().cloned().unwrap_or_default();

    // 构建代理配置
    let proxy_config = config.proxy_url.as_ref().map(|url| {
        let mut proxy = http_client::ProxyConfig::new(url);
        if let (Some(username), Some(password)) = (&config.proxy_username, &config.proxy_password) {
            proxy = proxy.with_auth(username, password);
        }
        proxy
    });

    if proxy_config.is_some() {
        tracing::info!("已配置 HTTP 代理: {}", config.proxy_url.as_ref().unwrap());
    }

    // 创建 MultiTokenManager 和 KiroProvider
    let credentials_path_buf: std::path::PathBuf = credentials_path.into();
    let token_manager = MultiTokenManager::new(
        config.clone(),
        credentials_list,
        proxy_config.clone(),
        Some(credentials_path_buf.clone()),
    )
    .unwrap_or_else(|e| {
        tracing::error!("创建 Token 管理器失败: {}", e);
        std::process::exit(1);
    });
    let token_manager = Arc::new(token_manager);

    // 初始化 count_tokens 配置
    token::init_config(token::CountTokensConfig {
        api_url: config.count_tokens_api_url.clone(),
        api_key: config.count_tokens_api_key.clone(),
        auth_type: config.count_tokens_auth_type.clone(),
        proxy: proxy_config.clone(),
        tls_backend: config.tls_backend,
    });

    // 构建 Admin API 路由（如果配置了非空的 admin_api_key）
    // 安全检查：空字符串被视为未配置，防止空 key 绕过认证
    let admin_key_valid = config
        .admin_api_key
        .as_ref()
        .map(|k| !k.trim().is_empty())
        .unwrap_or(false);

    // 获取配置目录
    let config_dir = std::path::Path::new(&config_path)
        .parent()
        .unwrap_or(std::path::Path::new("."));

    // 创建 API Key 管理器（必需，用于 API 认证）
    let api_keys_path = config_dir.join("api_keys.json");
    let api_key_manager =
        Arc::new(admin::ApiKeyManager::new(&api_keys_path).unwrap_or_else(|e| {
            tracing::error!("创建 API Key 管理器失败: {}", e);
            std::process::exit(1);
        }));

    // 创建池管理器（可选）
    let pools_path = config_dir.join("pools.json");
    let pool_manager = match PoolManager::new(
        config.clone(),
        proxy_config.clone(),
        &pools_path,
        &credentials_path_buf,
    ) {
        Ok(pm) => {
            let pool_count = pm.pool_count();
            tracing::info!("池管理器已初始化，共 {} 个池", pool_count);
            Some(Arc::new(pm))
        }
        Err(e) => {
            tracing::warn!("池管理器初始化失败: {}，池管理功能不可用", e);
            None
        }
    };

    // 构建 Anthropic API 路由
    let kiro_provider = KiroProvider::with_proxy(token_manager.clone(), proxy_config.clone());
    let anthropic_app = anthropic::create_router(
        api_key_manager.clone(),
        Some(kiro_provider),
        first_credentials.profile_arn.clone(),
        pool_manager.clone(),
    );

    let app: axum::Router = if let Some(admin_key) = &config.admin_api_key {
        if admin_key.trim().is_empty() {
            tracing::warn!("admin_api_key 配置为空，Admin API 未启用");
            anthropic_app
        } else {
            let admin_service = admin::AdminService::new(token_manager.clone());
            let mut admin_state = admin::AdminState::new(
                admin_key,
                admin_service,
                config.clone(),
                &config_path,
                api_key_manager.clone(),
            );

            // 如果池管理器初始化成功，添加到 AdminState
            if let Some(ref pm) = pool_manager {
                admin_state = admin_state.with_pool_manager(pm.clone());
            }

            let admin_app = admin::create_admin_router(admin_state);

            // 创建 Admin UI 路由
            let admin_ui_app = admin_ui::create_admin_ui_router();

            tracing::info!("Admin API 已启用");
            tracing::info!("Admin UI 已启用: /admin");
            tracing::info!("多 API Key 支持已启用（api_keys.json）");
            if pool_manager.is_some() {
                tracing::info!("API Key 绑定池路由已启用");
            }
            anthropic_app
                .nest("/api/admin", admin_app)
                .nest("/admin", admin_ui_app)
        }
    } else {
        anthropic_app
    };

    // 启动服务器
    let addr = format!("{}:{}", config.host, config.port);
    tracing::info!("启动服务: {}", addr);
    tracing::info!("API Key 认证已启用（api_keys.json）");
    tracing::info!("可用 API:");
    tracing::info!("  GET  /v1/models");
    tracing::info!("  POST /v1/messages");
    tracing::info!("  POST /v1/messages/count_tokens");

    if admin_key_valid {
        tracing::info!("Admin API:");
        tracing::info!("  GET  /api/admin/credentials");
        tracing::info!("  POST /api/admin/credentials/:id/disabled");
        tracing::info!("  POST /api/admin/credentials/:id/priority");
        tracing::info!("  POST /api/admin/credentials/:id/reset");
        tracing::info!("  GET  /api/admin/credentials/:id/balance");
        tracing::info!("  POST /api/admin/credentials/:id/pool");
        tracing::info!("  GET  /api/admin/pools");
        tracing::info!("  POST /api/admin/pools");
        tracing::info!("  GET  /api/admin/pools/:id");
        tracing::info!("  PUT  /api/admin/pools/:id");
        tracing::info!("  DELETE /api/admin/pools/:id");
        tracing::info!("  POST /api/admin/pools/:id/disabled");
        tracing::info!("Admin UI:");
        tracing::info!("  GET  /admin");
    }

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
