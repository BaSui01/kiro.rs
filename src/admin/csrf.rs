//! CSRF 保护模块
//!
//! 提供 CSRF Token 的生成、验证和管理功能

use parking_lot::RwLock;
use rand::Rng;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

/// CSRF Token 管理器
pub struct CsrfManager {
    /// Token 存储：token -> 过期时间戳（秒）
    tokens: RwLock<HashMap<String, i64>>,
    /// Token 有效期（秒）
    ttl_secs: i64,
    /// 操作计数器（用于定期清理）
    operation_count: AtomicU64,
    /// 清理间隔（每 N 次操作清理一次）
    cleanup_interval: u64,
}

impl CsrfManager {
    /// 创建新的 CSRF 管理器
    ///
    /// # 参数
    /// - `ttl_secs`: Token 有效期（秒）
    pub fn new(ttl_secs: i64) -> Self {
        Self {
            tokens: RwLock::new(HashMap::new()),
            ttl_secs,
            operation_count: AtomicU64::new(0),
            cleanup_interval: 100, // 每 100 次操作清理一次
        }
    }

    /// 生成新的 CSRF Token
    ///
    /// 返回一个 32 字节的十六进制字符串（64 字符）
    pub fn generate_token(&self) -> String {
        // 定期清理过期 Token
        self.maybe_cleanup();

        let mut rng = rand::thread_rng();
        let token_bytes: [u8; 32] = rng.r#gen();
        let token = hex::encode(token_bytes);

        let expires_at = chrono::Utc::now().timestamp() + self.ttl_secs;

        let mut tokens = self.tokens.write();
        tokens.insert(token.clone(), expires_at);

        token
    }

    /// 验证 CSRF Token
    ///
    /// 验证成功后会删除该 Token（一次性使用）
    pub fn validate_token(&self, token: &str) -> bool {
        // 定期清理过期 Token
        self.maybe_cleanup();

        let now = chrono::Utc::now().timestamp();

        let mut tokens = self.tokens.write();

        if let Some(&expires_at) = tokens.get(token) {
            if expires_at > now {
                // Token 有效，删除它（一次性使用）
                tokens.remove(token);
                return true;
            } else {
                // Token 已过期，删除它
                tokens.remove(token);
            }
        }

        false
    }

    /// 清理过期的 Token
    pub fn cleanup_expired(&self) {
        let now = chrono::Utc::now().timestamp();
        let mut tokens = self.tokens.write();
        let before_count = tokens.len();
        tokens.retain(|_, expires_at| *expires_at > now);
        let after_count = tokens.len();
        if before_count > after_count {
            tracing::debug!(
                "CSRF Token 清理: 删除 {} 个过期 Token，剩余 {} 个",
                before_count - after_count,
                after_count
            );
        }
    }

    /// 定期清理（内部使用）
    fn maybe_cleanup(&self) {
        let count = self.operation_count.fetch_add(1, Ordering::Relaxed);
        if count % self.cleanup_interval == 0 {
            self.cleanup_expired();
        }
    }

    /// 获取当前 Token 数量（用于监控）
    pub fn token_count(&self) -> usize {
        self.tokens.read().len()
    }

    /// 设置清理间隔（仅用于测试）
    #[cfg(test)]
    pub fn set_cleanup_interval(&mut self, interval: u64) {
        self.cleanup_interval = interval;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_and_validate() {
        let manager = CsrfManager::new(3600);
        let token = manager.generate_token();

        // Token 应该是 64 字符的十六进制字符串
        assert_eq!(token.len(), 64);

        // 第一次验证应该成功
        assert!(manager.validate_token(&token));

        // 第二次验证应该失败（一次性使用）
        assert!(!manager.validate_token(&token));
    }

    #[test]
    fn test_invalid_token() {
        let manager = CsrfManager::new(3600);
        assert!(!manager.validate_token("invalid_token"));
    }

    #[test]
    fn test_cleanup_expired() {
        let manager = CsrfManager::new(-1); // 立即过期
        let token = manager.generate_token();

        manager.cleanup_expired();

        // 过期的 Token 应该被清理
        assert!(!manager.validate_token(&token));
    }

    #[test]
    fn test_auto_cleanup() {
        let mut manager = CsrfManager::new(-1); // 立即过期
        manager.set_cleanup_interval(5); // 每 5 次操作清理一次

        // 生成 10 个 Token（会触发 2 次清理）
        for _ in 0..10 {
            manager.generate_token();
        }

        // 由于自动清理，过期的 Token 应该被清理掉
        // 只有最后几个可能还在（取决于清理时机）
        assert!(manager.token_count() < 10);
    }
}
