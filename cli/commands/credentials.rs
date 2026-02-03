//! 凭据管理命令

use anyhow::{Context, Result};
use serde_json;
use std::fs;
use std::path::Path;

use kiro_rs::kiro::model::credentials::{CredentialsConfig, KiroCredentials};

/// 列出所有凭据
pub async fn list(file: &str) -> Result<()> {
    let path = Path::new(file);

    if !path.exists() {
        println!("凭据文件不存在: {}", file);
        println!("使用 'credentials add' 命令添加凭据");
        return Ok(());
    }

    let config = CredentialsConfig::load(path)
        .with_context(|| format!("加载凭据文件失败: {}", file))?;

    let credentials = config.into_sorted_credentials();

    if credentials.is_empty() {
        println!("没有找到凭据");
        return Ok(());
    }

    println!("共 {} 个凭据:\n", credentials.len());

    for cred in credentials {
        let id = cred.id.unwrap_or(0);
        let auth_method = cred.auth_method.as_deref().unwrap_or("unknown");
        let priority = cred.priority;
        let region = cred.region.as_deref().unwrap_or("default");
        let pool_id = cred.pool_id.as_deref().unwrap_or("default");

        println!("ID: {}", id);
        println!("  认证方式: {}", auth_method);
        println!("  优先级: {}", priority);
        println!("  Region: {}", region);
        println!("  池 ID: {}", pool_id);

        if let Some(ref expires_at) = cred.expires_at {
            println!("  过期时间: {}", expires_at);
        }

        if let Some(ref profile_arn) = cred.profile_arn {
            println!("  Profile ARN: {}", profile_arn);
        }

        // 统计信息
        println!("  成功调用: {}", cred.success_count);
        println!("  失败调用: {}", cred.total_failure_count);

        if let Some(last_call) = cred.last_call_time {
            let datetime = chrono::DateTime::from_timestamp_millis(last_call as i64)
                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                .unwrap_or_else(|| "无效时间".to_string());
            println!("  最后调用: {}", datetime);
        }

        println!();
    }

    Ok(())
}

/// 添加新凭据
pub async fn add(
    file: &str,
    refresh_token: String,
    auth_method: String,
    priority: u32,
    region: Option<String>,
    client_id: Option<String>,
    client_secret: Option<String>,
) -> Result<()> {
    let path = Path::new(file);

    // 加载现有凭据
    let mut credentials = if path.exists() {
        let config = CredentialsConfig::load(path)
            .with_context(|| format!("加载凭据文件失败: {}", file))?;
        config.into_sorted_credentials()
    } else {
        Vec::new()
    };

    // 生成新 ID
    let new_id = credentials
        .iter()
        .filter_map(|c| c.id)
        .max()
        .unwrap_or(0)
        + 1;

    // 创建新凭据
    let new_cred = KiroCredentials {
        id: Some(new_id),
        access_token: None,
        refresh_token: Some(refresh_token),
        profile_arn: None,
        expires_at: None,
        auth_method: Some(auth_method.clone()),
        client_id,
        client_secret,
        priority,
        region,
        machine_id: None,
        pool_id: None,
        proxy_url: None,
        proxy_username: None,
        proxy_password: None,
        success_count: 0,
        total_failure_count: 0,
        last_call_time: None,
        total_response_time_ms: 0,
        token_refresh_count: 0,
        token_refresh_failure_count: 0,
        last_token_refresh_time: None,
    };

    credentials.push(new_cred);

    // 保存凭据
    save_credentials(path, &credentials)?;

    println!("凭据添加成功!");
    println!("ID: {}", new_id);
    println!("认证方式: {}", auth_method);
    println!("优先级: {}", priority);

    Ok(())
}

/// 删除凭据
pub async fn delete(file: &str, id: u64) -> Result<()> {
    let path = Path::new(file);

    if !path.exists() {
        anyhow::bail!("凭据文件不存在: {}", file);
    }

    let config = CredentialsConfig::load(path)
        .with_context(|| format!("加载凭据文件失败: {}", file))?;

    let mut credentials = config.into_sorted_credentials();

    let original_len = credentials.len();
    credentials.retain(|c| c.id != Some(id));

    if credentials.len() == original_len {
        anyhow::bail!("未找到 ID 为 {} 的凭据", id);
    }

    save_credentials(path, &credentials)?;

    println!("凭据删除成功! ID: {}", id);

    Ok(())
}

/// 导入凭据
pub async fn import(input: &str, output: &str, format: &str) -> Result<()> {
    let input_path = Path::new(input);

    if !input_path.exists() {
        anyhow::bail!("导入文件不存在: {}", input);
    }

    let content = fs::read_to_string(input_path)
        .with_context(|| format!("读取导入文件失败: {}", input))?;

    let imported_credentials: Vec<KiroCredentials> = match format {
        "json" => serde_json::from_str(&content)
            .with_context(|| format!("解析 JSON 文件失败: {}", input))?,
        "yaml" | "yml" => serde_yaml::from_str(&content)
            .with_context(|| format!("解析 YAML 文件失败: {}", input))?,
        _ => anyhow::bail!("不支持的格式: {}，支持 json 或 yaml", format),
    };

    if imported_credentials.is_empty() {
        println!("导入文件中没有凭据");
        return Ok(());
    }

    let output_path = Path::new(output);

    // 加载现有凭据
    let mut existing_credentials = if output_path.exists() {
        let config = CredentialsConfig::load(output_path)
            .with_context(|| format!("加载目标凭据文件失败: {}", output))?;
        config.into_sorted_credentials()
    } else {
        Vec::new()
    };

    // 获取当前最大 ID
    let mut next_id = existing_credentials
        .iter()
        .filter_map(|c| c.id)
        .max()
        .unwrap_or(0)
        + 1;

    // 合并凭据，为新凭据分配 ID
    let mut added_count = 0;
    for mut cred in imported_credentials {
        // 如果导入的凭据没有 ID 或 ID 已存在，分配新 ID
        if cred.id.is_none() || existing_credentials.iter().any(|c| c.id == cred.id) {
            cred.id = Some(next_id);
            next_id += 1;
        }
        existing_credentials.push(cred);
        added_count += 1;
    }

    // 保存合并后的凭据
    save_credentials(output_path, &existing_credentials)?;

    println!("导入成功! 共导入 {} 个凭据", added_count);
    println!("目标文件: {}", output);

    Ok(())
}

/// 导出凭据
pub async fn export(input: &str, output: &str, format: &str) -> Result<()> {
    let input_path = Path::new(input);

    if !input_path.exists() {
        anyhow::bail!("凭据文件不存在: {}", input);
    }

    let config = CredentialsConfig::load(input_path)
        .with_context(|| format!("加载凭据文件失败: {}", input))?;

    let credentials = config.into_sorted_credentials();

    if credentials.is_empty() {
        println!("没有凭据可导出");
        return Ok(());
    }

    let output_path = Path::new(output);

    // 确保输出目录存在
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("创建输出目录失败: {:?}", parent))?;
    }

    let content = match format {
        "json" => serde_json::to_string_pretty(&credentials)
            .with_context(|| "序列化为 JSON 失败")?,
        "yaml" | "yml" => {
            serde_yaml::to_string(&credentials).with_context(|| "序列化为 YAML 失败")?
        }
        _ => anyhow::bail!("不支持的格式: {}，支持 json 或 yaml", format),
    };

    fs::write(output_path, content)
        .with_context(|| format!("写入导出文件失败: {}", output))?;

    println!("导出成功! 共导出 {} 个凭据", credentials.len());
    println!("导出文件: {}", output);

    Ok(())
}

/// 保存凭据到文件
fn save_credentials(path: &Path, credentials: &[KiroCredentials]) -> Result<()> {
    // 确保目录存在
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("创建目录失败: {:?}", parent))?;
    }

    let content = serde_json::to_string_pretty(credentials)
        .with_context(|| "序列化凭据失败")?;

    fs::write(path, content)
        .with_context(|| format!("写入凭据文件失败: {:?}", path))?;

    Ok(())
}
