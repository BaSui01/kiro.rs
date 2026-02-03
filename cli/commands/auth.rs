//! OAuth 登录链接生成命令

use anyhow::Result;

/// 生成 OAuth 登录链接
pub async fn generate_login_link(
    auth_method: &str,
    region: &str,
    client_id: Option<String>,
) -> Result<()> {
    println!("生成 OAuth 登录链接:\n");

    match auth_method.to_lowercase().as_str() {
        "social" => {
            generate_social_login_link(region)?;
        }
        "idc" | "builder-id" | "iam" => {
            if let Some(cid) = client_id {
                generate_idc_login_link(region, &cid)?;
            } else {
                anyhow::bail!("IdC 认证需要提供 --client-id 参数");
            }
        }
        _ => {
            anyhow::bail!("不支持的认证方式: {}，支持 social 或 idc", auth_method);
        }
    }

    Ok(())
}

/// 生成 Social 认证登录链接
fn generate_social_login_link(region: &str) -> Result<()> {
    println!("认证方式: Social");
    println!("Region: {}", region);
    println!();

    // Social 认证使用 OAuth 流程
    let auth_url = format!(
        "https://prod.{}.auth.desktop.kiro.dev/authorize",
        region
    );

    println!("步骤 1: 在浏览器中打开以下链接:");
    println!("{}", auth_url);
    println!();

    println!("步骤 2: 使用您的 AWS 账户登录");
    println!();

    println!("步骤 3: 授权后，您将获得 refresh_token");
    println!();

    println!("步骤 4: 使用以下命令添加凭据:");
    println!("  kiro-cli credentials add \\");
    println!("    --refresh-token <YOUR_REFRESH_TOKEN> \\");
    println!("    --auth-method social \\");
    println!("    --region {}", region);
    println!();

    println!("注意事项:");
    println!("  - 确保复制完整的 refresh_token，不要截断");
    println!("  - refresh_token 通常很长 (200+ 字符)");
    println!("  - 如果 token 被截断，将无法正常使用");

    Ok(())
}

/// 生成 IdC 认证登录链接
fn generate_idc_login_link(region: &str, client_id: &str) -> Result<()> {
    println!("认证方式: IdC (Identity Center)");
    println!("Region: {}", region);
    println!("Client ID: {}", client_id);
    println!();

    // IdC 认证使用 OIDC 流程
    let auth_url = format!(
        "https://oidc.{}.amazonaws.com/authorize",
        region
    );

    let redirect_uri = "http://127.0.0.1:8080/oauth/callback";
    let response_type = "code";
    let scope = "openid profile";

    let full_url = format!(
        "{}?client_id={}&redirect_uri={}&response_type={}&scope={}",
        auth_url,
        urlencoding::encode(client_id),
        urlencoding::encode(redirect_uri),
        response_type,
        urlencoding::encode(scope)
    );

    println!("步骤 1: 在浏览器中打开以下链接:");
    println!("{}", full_url);
    println!();

    println!("步骤 2: 使用您的 AWS Builder ID 登录");
    println!();

    println!("步骤 3: 授权后，您将被重定向到回调 URL");
    println!("  回调 URL: {}", redirect_uri);
    println!();

    println!("步骤 4: 从回调 URL 中提取 authorization code");
    println!("  示例: http://127.0.0.1:8080/oauth/callback?code=<AUTHORIZATION_CODE>");
    println!();

    println!("步骤 5: 使用 authorization code 交换 refresh_token");
    println!("  (这一步需要调用 OIDC token endpoint)");
    println!();

    println!("步骤 6: 使用以下命令添加凭据:");
    println!("  kiro-cli credentials add \\");
    println!("    --refresh-token <YOUR_REFRESH_TOKEN> \\");
    println!("    --auth-method idc \\");
    println!("    --region {} \\", region);
    println!("    --client-id {} \\", client_id);
    println!("    --client-secret <YOUR_CLIENT_SECRET>");
    println!();

    println!("注意事项:");
    println!("  - IdC 认证需要 clientId 和 clientSecret");
    println!("  - 确保在 AWS IAM Identity Center 中正确配置了应用");
    println!("  - refresh_token 的获取需要完整的 OIDC 流程");

    Ok(())
}
