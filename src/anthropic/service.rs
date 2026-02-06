//! Anthropic API 业务逻辑服务
//!
//! 提取自 handlers.rs 的业务逻辑，包括：
//! - 请求验证和转换
//! - Token 计数
//! - 会话标识提取
//! - 流式/非流式响应处理

use std::sync::Arc;

use axum::http::HeaderMap;
use sha2::{Digest, Sha256};

use crate::kiro::model::requests::kiro::KiroRequest;
use crate::kiro::provider::KiroProvider;
use crate::token;

use super::converter::{ConversionError, ConversionResult, convert_request};
use super::history::{HistoryConfig, manage_history};
use super::types::MessagesRequest;
use super::websearch;

/// 上下文窗口大小（200k tokens）
pub const CONTEXT_WINDOW_SIZE: i32 = 200_000;

/// Ping 事件间隔（25秒）
pub const PING_INTERVAL_SECS: u64 = 25;

/// 请求处理上下文
///
/// 包含处理请求所需的所有信息
pub struct RequestContext {
    /// KiroProvider 实例
    pub provider: Arc<KiroProvider>,
    /// 序列化后的 Kiro 请求体
    pub request_body: String,
    /// 模型名称
    pub model: String,
    /// 估算的输入 tokens
    pub input_tokens: i32,
    /// 是否启用 thinking
    pub thinking_enabled: bool,
    /// 会话标识（用于粘性会话轮询）
    pub session_id: Option<String>,
    /// 是否为流式请求
    pub is_stream: bool,
}

/// 请求验证结果
pub enum ValidationResult {
    /// 验证通过，返回请求上下文
    Ok(RequestContext),
    /// Provider 未配置
    ProviderNotConfigured,
    /// WebSearch 请求，需要特殊处理
    WebSearchRequest {
        provider: Arc<KiroProvider>,
        input_tokens: i32,
    },
    /// 请求转换失败
    ConversionFailed(ConversionError),
    /// 序列化失败
    #[allow(dead_code)]
    SerializationFailed(String),
}

/// 从请求中提取会话标识
///
/// 优先级：
/// 1. metadata.user_id 中的 session_xxx（Claude Code 自带）
/// 2. x-session-id header（自定义）
/// 3. system prompt 哈希（兜底）
pub fn extract_session_id(req: &MessagesRequest, headers: &HeaderMap) -> Option<String> {
    // 优先级 1: metadata.user_id 中的 session
    // 格式: user_xxx_account__session_0b4445e1-f5be-49e1-87ce-62bbc28ad705
    if let Some(ref metadata) = req.metadata {
        if let Some(ref user_id) = metadata.user_id {
            if let Some(pos) = user_id.find("session_") {
                let session_part = &user_id[pos..];
                // 取 session_xxx 部分（到下一个 __ 或结尾）
                let end = session_part.find("__").unwrap_or(session_part.len());
                return Some(session_part[..end].to_string());
            }
        }
    }

    // 优先级 2: x-session-id header
    if let Some(session_id) = headers.get("x-session-id") {
        if let Ok(s) = session_id.to_str() {
            if !s.is_empty() {
                return Some(s.to_string());
            }
        }
    }

    // 优先级 3: system prompt 哈希（兜底）
    // ⚠️ 注意：哈希碰撞概率极低但存在，生产环境建议使用显式 session_id
    if let Some(ref system) = req.system {
        let content: String = system.iter().map(|s| s.text.as_str()).collect();
        if !content.is_empty() {
            let mut hasher = Sha256::new();
            hasher.update(content.as_bytes());
            let hash = hasher.finalize();
            let session_id = format!(
                "sys_{:x}",
                &hash[..8].iter().fold(0u64, |acc, &b| acc << 8 | b as u64)
            );
            tracing::debug!(
                "使用 system prompt 哈希作为会话标识: {} (长度={}字符)。\
                 建议：生产环境请使用 x-session-id header 或 metadata.user_id 中的显式 session_id",
                &session_id,
                content.len()
            );
            return Some(session_id);
        }
    }

    None
}

/// 估算输入 tokens
pub fn estimate_input_tokens(payload: &MessagesRequest) -> i32 {
    token::count_all_tokens(
        payload.model.clone(),
        payload.system.clone(),
        payload.messages.clone(),
        payload.tools.clone(),
    ) as i32
}

/// 检查是否启用了 thinking
pub fn is_thinking_enabled(payload: &MessagesRequest) -> bool {
    payload
        .thinking
        .as_ref()
        .map(|t| t.thinking_type == "enabled")
        .unwrap_or(false)
}

/// 检查是否为 WebSearch 请求
pub fn is_websearch_request(payload: &MessagesRequest) -> bool {
    websearch::has_web_search_tool(payload)
}

/// 转换请求并构建 Kiro 请求体
pub fn convert_and_build_request(
    payload: &MessagesRequest,
    profile_arn: Option<&str>,
    config: &crate::model::config::Config,
) -> Result<(String, ConversionResult), ConversionError> {
    // 应用历史管理（如果启用）
    let managed_payload = apply_history_management(payload, config);

    // 转换请求
    let conversion_result = convert_request(&managed_payload)?;

    // 构建 Kiro 请求
    let kiro_request = KiroRequest {
        conversation_state: conversion_result.conversation_state.clone(),
        profile_arn: profile_arn.map(|s| s.to_string()),
    };

    // 序列化
    let request_body = serde_json::to_string(&kiro_request)
        .map_err(|e| ConversionError::UnsupportedModel(format!("序列化失败: {}", e)))?;

    Ok((request_body, conversion_result))
}

/// 应用历史管理策略
///
/// 根据配置对消息历史进行智能管理，包括：
/// - 自动截断
/// - AI 摘要
/// - 图片占位符
/// - 缓存复用
fn apply_history_management(
    payload: &MessagesRequest,
    config: &crate::model::config::Config,
) -> MessagesRequest {
    // 创建历史管理配置
    let history_config = HistoryConfig {
        enabled: config.history_management_enabled,
        truncate_threshold: config.history_truncate_threshold,
        enable_ai_summary: config.history_enable_ai_summary,
        enable_image_placeholder: config.history_enable_image_placeholder,
        enable_prompt_caching: false, // 暂未实现
        keep_recent_messages: config.history_keep_recent_messages,
    };

    // 应用历史管理
    let result = manage_history(
        &history_config,
        payload.messages.clone(),
        payload.system.clone(),
        payload.tools.as_ref(),
    );

    // 记录处理结果
    if result.truncated || result.summarized || result.image_placeholder_applied {
        tracing::info!(
            "历史管理应用：truncated={}, summarized={}, image_placeholder={}, tokens: {} -> {}",
            result.truncated,
            result.summarized,
            result.image_placeholder_applied,
            result.original_tokens,
            result.processed_tokens
        );
    }

    // 返回处理后的请求
    MessagesRequest {
        model: payload.model.clone(),
        max_tokens: payload.max_tokens,
        messages: result.messages,
        stream: payload.stream,
        system: result.system,
        tools: payload.tools.clone(),
        tool_choice: payload.tool_choice.clone(),
        thinking: payload.thinking.clone(),
        output_config: payload.output_config.clone(),
        metadata: payload.metadata.clone(),
    }
}

/// 验证并准备请求
///
/// 执行以下步骤：
/// 1. 检查 KiroProvider 是否可用
/// 2. 检查是否为 WebSearch 请求
/// 3. 转换请求格式
/// 4. 构建 Kiro 请求体
/// 5. 估算 Token 数量
pub fn validate_and_prepare_request(
    provider: Option<&Arc<KiroProvider>>,
    profile_arn: Option<&String>,
    payload: &MessagesRequest,
    headers: &HeaderMap,
    config: &crate::model::config::Config,
) -> ValidationResult {
    // 检查 KiroProvider 是否可用
    let provider = match provider {
        Some(p) => p.clone(),
        None => {
            tracing::error!("KiroProvider 未配置");
            return ValidationResult::ProviderNotConfigured;
        }
    };

    // 检查是否为 WebSearch 请求
    if is_websearch_request(payload) {
        tracing::info!("检测到 WebSearch 工具，路由到 WebSearch 处理");
        let input_tokens = estimate_input_tokens(payload);
        return ValidationResult::WebSearchRequest {
            provider,
            input_tokens,
        };
    }

    // 转换请求
    let (request_body, _conversion_result) = match convert_and_build_request(payload, profile_arn.map(|s| s.as_str()), config) {
        Ok(result) => result,
        Err(e) => {
            tracing::warn!("请求转换失败: {}", e);
            return ValidationResult::ConversionFailed(e);
        }
    };

    tracing::debug!("Kiro request body: {}", request_body);

    // 估算输入 tokens
    let input_tokens = estimate_input_tokens(payload);

    // 检查是否启用了 thinking
    let thinking_enabled = is_thinking_enabled(payload);

    // 提取会话标识
    let session_id = extract_session_id(payload, headers);

    ValidationResult::Ok(RequestContext {
        provider,
        request_body,
        model: payload.model.clone(),
        input_tokens,
        thinking_enabled,
        session_id,
        is_stream: payload.stream,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anthropic::types::{Metadata, SystemMessage, Thinking};

    #[test]
    fn test_extract_session_id_from_metadata() {
        let req = MessagesRequest {
            model: "claude-3-opus".to_string(),
            max_tokens: 1024,
            messages: vec![],
            stream: false,
            system: None,
            tools: None,
            thinking: None,
            metadata: Some(Metadata {
                user_id: Some(
                    "user_xxx_account__session_0b4445e1-f5be-49e1-87ce-62bbc28ad705".to_string(),
                ),
            }),
            tool_choice: None,
        };

        let headers = HeaderMap::new();
        let session_id = extract_session_id(&req, &headers);

        assert!(session_id.is_some());
        assert!(session_id.unwrap().starts_with("session_"));
    }

    #[test]
    fn test_extract_session_id_from_header() {
        let req = MessagesRequest {
            model: "claude-3-opus".to_string(),
            max_tokens: 1024,
            messages: vec![],
            stream: false,
            system: None,
            tools: None,
            thinking: None,
            metadata: None,
            tool_choice: None,
        };

        let mut headers = HeaderMap::new();
        headers.insert("x-session-id", "my-custom-session".parse().unwrap());

        let session_id = extract_session_id(&req, &headers);

        assert_eq!(session_id, Some("my-custom-session".to_string()));
    }

    #[test]
    fn test_extract_session_id_from_system_hash() {
        let req = MessagesRequest {
            model: "claude-3-opus".to_string(),
            max_tokens: 1024,
            messages: vec![],
            stream: false,
            system: Some(vec![SystemMessage {
                text: "You are a helpful assistant.".to_string(),
            }]),
            tools: None,
            thinking: None,
            metadata: None,
            tool_choice: None,
        };

        let headers = HeaderMap::new();
        let session_id = extract_session_id(&req, &headers);

        assert!(session_id.is_some());
        assert!(session_id.unwrap().starts_with("sys_"));
    }

    #[test]
    fn test_is_thinking_enabled() {
        let mut req = MessagesRequest {
            model: "claude-3-opus".to_string(),
            max_tokens: 1024,
            messages: vec![],
            stream: false,
            system: None,
            tools: None,
            thinking: None,
            metadata: None,
            tool_choice: None,
        };

        // 未启用
        assert!(!is_thinking_enabled(&req));

        // 启用
        req.thinking = Some(Thinking {
            thinking_type: "enabled".to_string(),
            budget_tokens: 1000,
        });
        assert!(is_thinking_enabled(&req));

        // 禁用
        req.thinking = Some(Thinking {
            thinking_type: "disabled".to_string(),
            budget_tokens: 0,
        });
        assert!(!is_thinking_enabled(&req));
    }
}
