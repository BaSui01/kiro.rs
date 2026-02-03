//! Token 扫描和验证命令

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use std::path::Path;

use kiro_rs::http_client::ProxyConfig;
use kiro_rs::kiro::model::credentials::CredentialsConfig;
use kiro_rs::kiro::token_manager::{is_token_expired, is_token_expiring_soon, refresh_token};
use kiro_rs::model::config::Config;

/// 扫描本地 Token
pub async fn scan(file: &str) -> Result<()> {
    let path = Path::new(file);

    if !path.exists() {
        println!("凭据文件不存在: {}", file);
        return Ok(());
    }

    let config = CredentialsConfig::load(path)
        .with_context(|| format!("加载凭据文件失败: {}", file))?;

    let credentials = config.into_sorted_credentials();

    if credentials.is_empty() {
        println!("没有找到凭据");
        return Ok(());
    }

    println!("扫描到 {} 个 Token:\n", credentials.len());

    for cred in credentials {
        let id = cred.id.unwrap_or(0);
        let auth_method = cred.auth_method.as_deref().unwrap_or("unknown");

        println!("ID: {}", id);
        println!("  认证方式: {}", auth_method);

        // 检查 refresh_token
        if let Some(ref refresh_token) = cred.refresh_token {
            let token_len = refresh_token.len();
            let token_preview = if token_len > 20 {
                format!("{}...{}", &refresh_token[..10], &refresh_token[token_len - 10..])
            } else {
                refresh_token.clone()
            };
            println!("  Refresh Token: {} (长度: {})", token_preview, token_len);

            // 检查 token 是否被截断
            if token_len < 100 || refresh_token.ends_with("...") || refresh_token.contains("...") {
                println!("  ⚠️  警告: Token 可能已被截断");
            }
        } else {
            println!("  Refresh Token: 未设置");
        }

        // 检查 access_token
        if let Some(ref access_token) = cred.access_token {
            let token_len = access_token.len();
            println!("  Access Token: 已设置 (长度: {})", token_len);
        } else {
            println!("  Access Token: 未设置");
        }

        // 检查过期时间
        if let Some(ref expires_at) = cred.expires_at {
            println!("  过期时间: {}", expires_at);

            // 解析过期时间并判断状态
            if let Ok(expires) = DateTime::parse_from_rfc3339(expires_at) {
                let now = Utc::now();
                if expires <= now {
                    println!("  状态: ❌ 已过期");
                } else {
                    let duration = expires.signed_duration_since(now);
                    let hours = duration.num_hours();
                    let minutes = duration.num_minutes() % 60;

                    if hours < 1 {
                        println!("  状态: ⚠️  即将过期 (剩余 {} 分钟)", minutes);
                    } else if hours < 24 {
                        println!("  状态: ✓ 有效 (剩余 {} 小时 {} 分钟)", hours, minutes);
                    } else {
                        let days = duration.num_days();
                        println!("  状态: ✓ 有效 (剩余 {} 天)", days);
                    }
                }
            }
        } else {
            println!("  过期时间: 未设置");
            println!("  状态: ⚠️  需要刷新");
        }

        // IdC 认证信息
        if auth_method.eq_ignore_ascii_case("idc") {
            if cred.client_id.is_some() && cred.client_secret.is_some() {
                println!("  IdC 配置: ✓ 已配置");
            } else {
                println!("  IdC 配置: ❌ 缺少 clientId 或 clientSecret");
            }
        }

        println!();
    }

    Ok(())
}

/// 验证 Token 有效性
pub async fn validate(file: &str, config_file: &str, id: Option<u64>) -> Result<()> {
    let path = Path::new(file);

    if !path.exists() {
        anyhow::bail!("凭据文件不存在: {}", file);
    }

    let _config = Config::load(config_file)
        .with_context(|| format!("加载配置文件失败: {}", config_file))?;

    let creds_config = CredentialsConfig::load(path)
        .with_context(|| format!("加载凭据文件失败: {}", file))?;

    let credentials = creds_config.into_sorted_credentials();

    if credentials.is_empty() {
        println!("没有找到凭据");
        return Ok(());
    }

    // 过滤要验证的凭据
    let to_validate: Vec<_> = if let Some(target_id) = id {
        credentials
            .into_iter()
            .filter(|c| c.id == Some(target_id))
            .collect()
    } else {
        credentials
    };

    if to_validate.is_empty() {
        if let Some(target_id) = id {
            anyhow::bail!("未找到 ID 为 {} 的凭据", target_id);
        } else {
            println!("没有凭据需要验证");
            return Ok(());
        }
    }

    println!("验证 {} 个凭据:\n", to_validate.len());

    for cred in to_validate {
        let cred_id = cred.id.unwrap_or(0);
        println!("ID: {}", cred_id);

        // 检查 refresh_token
        if cred.refresh_token.is_none() {
            println!("  ❌ 验证失败: 缺少 refresh_token");
            println!();
            continue;
        }

        let refresh_token = cred.refresh_token.as_ref().unwrap();

        // 检查 token 长度
        if refresh_token.len() < 100 {
            println!("  ❌ 验证失败: refresh_token 长度不足 ({})", refresh_token.len());
            println!();
            continue;
        }

        // 检查 token 是否被截断
        if refresh_token.ends_with("...") || refresh_token.contains("...") {
            println!("  ❌ 验证失败: refresh_token 已被截断");
            println!();
            continue;
        }

        // 检查过期状态
        if is_token_expired(&cred) {
            println!("  ⚠️  Token 已过期或即将过期，需要刷新");
        } else if is_token_expiring_soon(&cred) {
            println!("  ⚠️  Token 即将过期 (10分钟内)");
        } else if let Some(ref expires_at) = cred.expires_at {
            if let Ok(expires) = DateTime::parse_from_rfc3339(expires_at) {
                let duration = expires.signed_duration_since(Utc::now());
                let hours = duration.num_hours();
                println!("  ✓ Token 有效 (剩余 {} 小时)", hours);
            } else {
                println!("  ⚠️  无法解析过期时间");
            }
        } else {
            println!("  ⚠️  未设置过期时间，需要刷新");
        }

        // IdC 认证检查
        let auth_method = cred.auth_method.as_deref().unwrap_or("social");
        if auth_method.eq_ignore_ascii_case("idc") {
            if cred.client_id.is_none() || cred.client_secret.is_none() {
                println!("  ❌ IdC 认证缺少 clientId 或 clientSecret");
            } else {
                println!("  ✓ IdC 认证配置完整");
            }
        }

        println!();
    }

    Ok(())
}

/// 刷新 Token
pub async fn refresh(file: &str, config_file: &str, id: Option<u64>) -> Result<()> {
    let path = Path::new(file);

    if !path.exists() {
        anyhow::bail!("凭据文件不存在: {}", file);
    }

    let config = Config::load(config_file)
        .with_context(|| format!("加载配置文件失败: {}", config_file))?;

    let creds_config = CredentialsConfig::load(path)
        .with_context(|| format!("加载凭据文件失败: {}", file))?;

    let mut credentials = creds_config.into_sorted_credentials();

    if credentials.is_empty() {
        println!("没有找到凭据");
        return Ok(());
    }

    // 构建代理配置
    let proxy_config = config.proxy_url.as_ref().map(|url| {
        let mut proxy = ProxyConfig::new(url);
        if let (Some(username), Some(password)) = (&config.proxy_username, &config.proxy_password) {
            proxy = proxy.with_auth(username, password);
        }
        proxy
    });

    // 过滤要刷新的凭据
    let indices_to_refresh: Vec<usize> = if let Some(target_id) = id {
        credentials
            .iter()
            .enumerate()
            .filter(|(_, c)| c.id == Some(target_id))
            .map(|(i, _)| i)
            .collect()
    } else {
        (0..credentials.len()).collect()
    };

    if indices_to_refresh.is_empty() {
        if let Some(target_id) = id {
            anyhow::bail!("未找到 ID 为 {} 的凭据", target_id);
        } else {
            println!("没有凭据需要刷新");
            return Ok(());
        }
    }

    println!("刷新 {} 个凭据:\n", indices_to_refresh.len());

    let mut success_count = 0;
    let mut failure_count = 0;

    for idx in indices_to_refresh {
        let cred = &credentials[idx];
        let cred_id = cred.id.unwrap_or(0);

        println!("ID: {}", cred_id);

        match refresh_token(cred, &config, proxy_config.as_ref()).await {
            Ok(refreshed_cred) => {
                println!("  ✓ 刷新成功");

                if let Some(ref expires_at) = refreshed_cred.expires_at {
                    println!("  新过期时间: {}", expires_at);
                }

                // 更新凭据
                credentials[idx] = refreshed_cred;
                success_count += 1;
            }
            Err(e) => {
                println!("  ❌ 刷新失败: {}", e);
                failure_count += 1;
            }
        }

        println!();
    }

    // 保存更新后的凭据
    if success_count > 0 {
        let content = serde_json::to_string_pretty(&credentials)
            .with_context(|| "序列化凭据失败")?;

        std::fs::write(path, content)
            .with_context(|| format!("写入凭据文件失败: {}", file))?;

        println!("凭据文件已更新");
    }

    println!("\n刷新完成:");
    println!("  成功: {}", success_count);
    println!("  失败: {}", failure_count);

    Ok(())
}
