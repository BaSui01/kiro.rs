//! Kiro.rs CLI Tool
//!
//! 命令行工具，用于管理凭据、扫描 Token、生成登录链接等

mod commands;
mod utils;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "kiro-cli")]
#[command(version, about = "Kiro.rs 命令行工具", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 凭据管理
    #[command(subcommand)]
    Credentials(CredentialsCommands),

    /// Token 扫描和验证
    #[command(subcommand)]
    Token(TokenCommands),

    /// OAuth 登录链接生成
    #[command(subcommand)]
    Auth(AuthCommands),
}

#[derive(Subcommand)]
enum CredentialsCommands {
    /// 列出所有凭据
    List {
        /// 凭据文件路径
        #[arg(short, long, default_value = "config/credentials.json")]
        file: String,
    },

    /// 添加新凭据
    Add {
        /// Refresh Token
        #[arg(short, long)]
        refresh_token: String,

        /// 认证方式 (social/idc)
        #[arg(short, long, default_value = "social")]
        auth_method: String,

        /// 优先级 (数字越小优先级越高)
        #[arg(short, long, default_value = "0")]
        priority: u32,

        /// Region
        #[arg(long)]
        region: Option<String>,

        /// Client ID (IdC 认证需要)
        #[arg(long)]
        client_id: Option<String>,

        /// Client Secret (IdC 认证需要)
        #[arg(long)]
        client_secret: Option<String>,

        /// 凭据文件路径
        #[arg(short, long, default_value = "config/credentials.json")]
        file: String,
    },

    /// 删除凭据
    Delete {
        /// 凭据 ID
        #[arg(short, long)]
        id: u64,

        /// 凭据文件路径
        #[arg(short, long, default_value = "config/credentials.json")]
        file: String,
    },

    /// 导入凭据
    Import {
        /// 导入文件路径
        #[arg(short, long)]
        input: String,

        /// 目标凭据文件路径
        #[arg(short, long, default_value = "config/credentials.json")]
        output: String,

        /// 文件格式 (json/yaml)
        #[arg(long, default_value = "json")]
        format: String,
    },

    /// 导出凭据
    Export {
        /// 凭据文件路径
        #[arg(short, long, default_value = "config/credentials.json")]
        input: String,

        /// 导出文件路径
        #[arg(short, long)]
        output: String,

        /// 文件格式 (json/yaml)
        #[arg(long, default_value = "json")]
        format: String,
    },
}

#[derive(Subcommand)]
enum TokenCommands {
    /// 扫描本地 Token
    Scan {
        /// 凭据文件路径
        #[arg(short, long, default_value = "config/credentials.json")]
        file: String,
    },

    /// 验证 Token 有效性
    Validate {
        /// 凭据文件路径
        #[arg(short, long, default_value = "config/credentials.json")]
        file: String,

        /// 配置文件路径
        #[arg(short, long, default_value = "config/config.json")]
        config: String,

        /// 凭据 ID (可选，不指定则验证所有凭据)
        #[arg(short, long)]
        id: Option<u64>,
    },

    /// 刷新 Token
    Refresh {
        /// 凭据文件路径
        #[arg(short, long, default_value = "config/credentials.json")]
        file: String,

        /// 配置文件路径
        #[arg(short, long, default_value = "config/config.json")]
        config: String,

        /// 凭据 ID (可选，不指定则刷新所有凭据)
        #[arg(short, long)]
        id: Option<u64>,
    },
}

#[derive(Subcommand)]
enum AuthCommands {
    /// 生成 OAuth 登录链接
    Login {
        /// 认证方式 (social/idc)
        #[arg(short, long, default_value = "social")]
        auth_method: String,

        /// Region
        #[arg(short, long, default_value = "us-east-1")]
        region: String,

        /// Client ID (IdC 认证需要)
        #[arg(long)]
        client_id: Option<String>,
    },
}

#[tokio::main]
async fn main() {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Credentials(cmd) => match cmd {
            CredentialsCommands::List { file } => commands::credentials::list(&file).await,
            CredentialsCommands::Add {
                refresh_token,
                auth_method,
                priority,
                region,
                client_id,
                client_secret,
                file,
            } => {
                commands::credentials::add(
                    &file,
                    refresh_token,
                    auth_method,
                    priority,
                    region,
                    client_id,
                    client_secret,
                )
                .await
            }
            CredentialsCommands::Delete { id, file } => {
                commands::credentials::delete(&file, id).await
            }
            CredentialsCommands::Import {
                input,
                output,
                format,
            } => commands::credentials::import(&input, &output, &format).await,
            CredentialsCommands::Export {
                input,
                output,
                format,
            } => commands::credentials::export(&input, &output, &format).await,
        },
        Commands::Token(cmd) => match cmd {
            TokenCommands::Scan { file } => commands::token::scan(&file).await,
            TokenCommands::Validate { file, config, id } => {
                commands::token::validate(&file, &config, id).await
            }
            TokenCommands::Refresh { file, config, id } => {
                commands::token::refresh(&file, &config, id).await
            }
        },
        Commands::Auth(cmd) => match cmd {
            AuthCommands::Login {
                auth_method,
                region,
                client_id,
            } => commands::auth::generate_login_link(&auth_method, &region, client_id).await,
        },
    };

    if let Err(e) = result {
        eprintln!("错误: {}", e);
        std::process::exit(1);
    }
}
