//! Admin API 模块
//!
//! 提供凭据管理、配置管理和 API Key 管理功能的 HTTP API
//!
//! # 功能
//! - 查询所有凭据状态
//! - 启用/禁用凭据
//! - 修改凭据优先级
//! - 重置失败计数
//! - 查询凭据余额
//! - 配置管理（读取/更新）
//! - API Key 管理（CRUD）
//! - 池管理（CRUD）
//!
//! # 使用
//! ```ignore
//! let admin_service = AdminService::new(token_manager.clone());
//! let admin_state = AdminState::new(admin_api_key, admin_service);
//! let admin_router = create_admin_router(admin_state);
//! ```

pub mod api_keys;
mod api_key_handlers;
mod config_handlers;
pub mod csrf;
mod error;
mod handlers;
mod middleware;
mod pool_handlers;
mod router;
mod service;
pub mod types;

pub use api_keys::ApiKeyManager;
pub use middleware::AdminState;
pub use router::create_admin_router;
pub use service::AdminService;
