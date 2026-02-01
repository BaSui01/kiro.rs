//! 凭证池错误类型定义

/// 池操作错误
#[derive(Debug, thiserror::Error)]
pub enum PoolError {
    /// 池不存在
    #[error("池不存在: {pool_id}")]
    PoolNotFound { pool_id: String },

    /// 池已存在
    #[error("池已存在: {pool_id}")]
    PoolAlreadyExists { pool_id: String },

    /// 不能删除默认池
    #[error("不能删除默认池")]
    CannotDeleteDefaultPool,

    /// 凭据不存在
    #[error("凭据不存在: {credential_id}")]
    CredentialNotFound { credential_id: u64 },

    /// 配置加载失败
    #[error("配置加载失败: {reason}")]
    ConfigLoadFailed { reason: String },

    /// IO 错误
    #[error("IO 错误: {0}")]
    IoError(#[from] std::io::Error),

    /// JSON 错误
    #[error("JSON 错误: {0}")]
    JsonError(#[from] serde_json::Error),

    /// Token 管理器错误
    #[error("Token 管理器错误: {0}")]
    TokenManagerError(String),
}

impl PoolError {
    /// 检查是否为"池不存在"错误
    pub fn is_pool_not_found(&self) -> bool {
        matches!(self, PoolError::PoolNotFound { .. })
    }

    /// 检查是否为"池已存在"错误
    pub fn is_pool_already_exists(&self) -> bool {
        matches!(self, PoolError::PoolAlreadyExists { .. })
    }

    /// 检查是否为"凭据不存在"错误
    pub fn is_credential_not_found(&self) -> bool {
        matches!(self, PoolError::CredentialNotFound { .. })
    }

    /// 检查是否为"不能删除默认池"错误
    pub fn is_cannot_delete_default_pool(&self) -> bool {
        matches!(self, PoolError::CannotDeleteDefaultPool)
    }
}
