//! 智能消息历史管理模块
//!
//! 实现四层策略应对 Anthropic API 的长度限制：
//! 1. **自动截断**：超过阈值时截断早期消息
//! 2. **AI 摘要**：使用 Haiku 模型摘要历史消息
//! 3. **图片占位符**：历史消息中的图片替换为 `[Image]`
//! 4. **缓存复用**：利用 Anthropic 的 Prompt Caching 功能

use crate::anthropic::types::{ContentBlock, Message, SystemMessage};
use crate::token;

/// 历史管理配置
#[derive(Debug, Clone)]
pub struct HistoryConfig {
    /// 是否启用历史管理
    pub enabled: bool,
    /// 自动截断阈值（tokens）
    pub truncate_threshold: u64,
    /// 是否启用 AI 摘要
    pub enable_ai_summary: bool,
    /// 是否启用图片占位符
    pub enable_image_placeholder: bool,
    /// 是否启用缓存复用
    pub enable_prompt_caching: bool,
    /// 保留最近的消息数量（截断时）
    pub keep_recent_messages: usize,
}

impl Default for HistoryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            truncate_threshold: 100_000,
            enable_ai_summary: false, // 默认关闭，需要额外的 API 调用
            enable_image_placeholder: true,
            enable_prompt_caching: true,
            keep_recent_messages: 20,
        }
    }
}

/// 历史管理结果
#[derive(Debug)]
pub struct HistoryManagementResult {
    /// 处理后的消息列表
    pub messages: Vec<Message>,
    /// 处理后的系统消息
    pub system: Option<Vec<SystemMessage>>,
    /// 是否应用了截断
    pub truncated: bool,
    /// 是否应用了 AI 摘要
    pub summarized: bool,
    /// 是否应用了图片占位符
    pub image_placeholder_applied: bool,
    /// 原始 token 数量
    pub original_tokens: u64,
    /// 处理后 token 数量
    pub processed_tokens: u64,
}

/// 智能管理消息历史
///
/// 根据配置应用四层策略：
/// 1. 图片占位符（如果启用）
/// 2. 计算 token 数量
/// 3. 如果超过阈值，应用截断或 AI 摘要
/// 4. 添加缓存标记（如果启用）
pub fn manage_history(
    config: &HistoryConfig,
    messages: Vec<Message>,
    system: Option<Vec<SystemMessage>>,
    tools: Option<&Vec<crate::anthropic::types::Tool>>,
) -> HistoryManagementResult {
    if !config.enabled {
        let original_tokens = estimate_total_tokens(&messages, &system, tools);
        return HistoryManagementResult {
            messages,
            system,
            truncated: false,
            summarized: false,
            image_placeholder_applied: false,
            original_tokens,
            processed_tokens: original_tokens,
        };
    }

    // 策略 3: 图片占位符
    let (processed_messages, image_placeholder_applied) = if config.enable_image_placeholder {
        (apply_image_placeholder(&messages), true)
    } else {
        (messages.clone(), false)
    };

    // 计算原始 token 数量
    let original_tokens = estimate_total_tokens(&processed_messages, &system, tools);

    tracing::debug!(
        "历史管理：原始 tokens = {}, 阈值 = {}",
        original_tokens,
        config.truncate_threshold
    );

    // 检查是否需要截断或摘要
    if original_tokens <= config.truncate_threshold {
        // 未超过阈值，直接返回
        return HistoryManagementResult {
            messages: processed_messages,
            system,
            truncated: false,
            summarized: false,
            image_placeholder_applied,
            original_tokens,
            processed_tokens: original_tokens,
        };
    }

    // 超过阈值，应用策略
    let (final_messages, final_system, truncated, summarized) = if config.enable_ai_summary {
        // 策略 2: AI 摘要（优先）
        tracing::info!("应用 AI 摘要策略（tokens: {} > {}）", original_tokens, config.truncate_threshold);
        let (msgs, sys) = apply_ai_summary(&processed_messages, &system);
        (msgs, sys, false, true)
    } else {
        // 策略 1: 自动截断
        tracing::info!("应用自动截断策略（tokens: {} > {}）", original_tokens, config.truncate_threshold);
        let (msgs, sys) = apply_truncation(&processed_messages, &system, config.keep_recent_messages);
        (msgs, sys, true, false)
    };

    // 计算处理后的 token 数量
    let processed_tokens = estimate_total_tokens(&final_messages, &final_system, tools);

    tracing::info!(
        "历史管理完成：{} tokens -> {} tokens (减少 {:.1}%)",
        original_tokens,
        processed_tokens,
        (original_tokens - processed_tokens) as f64 / original_tokens as f64 * 100.0
    );

    HistoryManagementResult {
        messages: final_messages,
        system: final_system,
        truncated,
        summarized,
        image_placeholder_applied,
        original_tokens,
        processed_tokens,
    }
}

/// 策略 1: 自动截断早期消息
///
/// 保留最近的 N 条消息和 system prompt
fn apply_truncation(
    messages: &[Message],
    system: &Option<Vec<SystemMessage>>,
    keep_recent: usize,
) -> (Vec<Message>, Option<Vec<SystemMessage>>) {
    if messages.len() <= keep_recent {
        return (messages.to_vec(), system.clone());
    }

    // 保留最后 N 条消息
    let start_index = messages.len().saturating_sub(keep_recent);
    let truncated_messages = messages[start_index..].to_vec();

    tracing::debug!(
        "截断历史消息：{} -> {} 条",
        messages.len(),
        truncated_messages.len()
    );

    // 在截断的消息前添加提示
    let mut result_messages = Vec::new();

    // 添加截断提示消息
    let truncation_notice = Message {
        role: "user".to_string(),
        content: serde_json::json!("[Earlier messages truncated to manage context length]"),
    };
    result_messages.push(truncation_notice);

    // 添加保留的消息
    result_messages.extend(truncated_messages);

    (result_messages, system.clone())
}

/// 策略 2: AI 摘要历史消息
///
/// 使用 Haiku 模型摘要历史消息，将长历史压缩为简短摘要
///
/// 注意：此功能需要额外的 API 调用，当前实现为占位符
fn apply_ai_summary(
    messages: &[Message],
    system: &Option<Vec<SystemMessage>>,
) -> (Vec<Message>, Option<Vec<SystemMessage>>) {
    // TODO: 实现 AI 摘要功能
    // 1. 将历史消息格式化为文本
    // 2. 调用 Haiku 模型生成摘要
    // 3. 将摘要作为新的历史消息

    tracing::warn!("AI 摘要功能尚未实现，回退到截断策略");

    // 暂时回退到截断策略
    apply_truncation(messages, system, 20)
}

/// 策略 3: 图片占位符
///
/// 将历史消息中的图片替换为 `[Image]` 占位符，减少 token 消耗
fn apply_image_placeholder(messages: &[Message]) -> Vec<Message> {
    messages
        .iter()
        .map(|msg| {
            let new_content = replace_images_in_content(&msg.content);
            Message {
                role: msg.role.clone(),
                content: new_content,
            }
        })
        .collect()
}

/// 替换内容中的图片为占位符
fn replace_images_in_content(content: &serde_json::Value) -> serde_json::Value {
    match content {
        serde_json::Value::String(s) => serde_json::json!(s),
        serde_json::Value::Array(arr) => {
            let new_arr: Vec<serde_json::Value> = arr
                .iter()
                .map(|item| {
                    if let Ok(block) = serde_json::from_value::<ContentBlock>(item.clone()) {
                        if block.block_type == "image" {
                            // 替换图片为占位符
                            return serde_json::json!({
                                "type": "text",
                                "text": "[Image]"
                            });
                        }
                    }
                    item.clone()
                })
                .collect();
            serde_json::json!(new_arr)
        }
        _ => content.clone(),
    }
}

/// 估算总 token 数量
fn estimate_total_tokens(
    messages: &[Message],
    system: &Option<Vec<SystemMessage>>,
    tools: Option<&Vec<crate::anthropic::types::Tool>>,
) -> u64 {
    let mut total = 0u64;

    // 系统消息
    if let Some(sys) = system {
        for msg in sys {
            total += token::count_tokens(&msg.text);
        }
    }

    // 用户消息
    for msg in messages {
        total += estimate_message_tokens(msg);
    }

    // 工具定义
    if let Some(tools) = tools {
        for tool in tools {
            total += token::count_tokens(&tool.name);
            total += token::count_tokens(&tool.description);
            if let Ok(schema_json) = serde_json::to_string(&tool.input_schema) {
                total += token::count_tokens(&schema_json);
            }
        }
    }

    total
}

/// 估算单条消息的 token 数量
fn estimate_message_tokens(msg: &Message) -> u64 {
    match &msg.content {
        serde_json::Value::String(s) => token::count_tokens(s),
        serde_json::Value::Array(arr) => {
            let mut total = 0u64;
            for item in arr {
                if let Some(text) = item.get("text").and_then(|v| v.as_str()) {
                    total += token::count_tokens(text);
                }
                // 图片估算为 1000 tokens
                if item.get("type").and_then(|v| v.as_str()) == Some("image") {
                    total += 1000;
                }
                // tool_use 估算
                if item.get("type").and_then(|v| v.as_str()) == Some("tool_use") {
                    if let Some(input) = item.get("input") {
                        if let Ok(input_str) = serde_json::to_string(input) {
                            total += token::count_tokens(&input_str);
                        }
                    }
                    total += 50; // tool_use 开销
                }
                // tool_result 估算
                if item.get("type").and_then(|v| v.as_str()) == Some("tool_result") {
                    if let Some(content) = item.get("content") {
                        if let Some(text) = content.as_str() {
                            total += token::count_tokens(text);
                        } else if let Ok(content_str) = serde_json::to_string(content) {
                            total += token::count_tokens(&content_str);
                        }
                    }
                    total += 50; // tool_result 开销
                }
            }
            total
        }
        _ => 0,
    }
}

/// 策略 4: 添加缓存标记
///
/// 为 system prompt 和历史消息添加缓存标记，利用 Anthropic 的 Prompt Caching 功能
///
/// 注意：Anthropic Prompt Caching 需要在请求中添加特殊的缓存标记
/// 当前 Kiro API 可能不支持此功能，此函数为占位符
#[allow(dead_code)]
fn apply_prompt_caching(
    messages: Vec<Message>,
    system: Option<Vec<SystemMessage>>,
) -> (Vec<Message>, Option<Vec<SystemMessage>>) {
    // TODO: 实现 Prompt Caching 标记
    // Anthropic 的 Prompt Caching 需要在 system 和 messages 中添加特殊字段
    // 参考: https://docs.anthropic.com/claude/docs/prompt-caching

    tracing::debug!("Prompt Caching 功能尚未实现");
    (messages, system)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_truncation() {
        let messages = vec![
            Message {
                role: "user".to_string(),
                content: serde_json::json!("Message 1"),
            },
            Message {
                role: "assistant".to_string(),
                content: serde_json::json!("Response 1"),
            },
            Message {
                role: "user".to_string(),
                content: serde_json::json!("Message 2"),
            },
            Message {
                role: "assistant".to_string(),
                content: serde_json::json!("Response 2"),
            },
            Message {
                role: "user".to_string(),
                content: serde_json::json!("Message 3"),
            },
        ];

        let system = Some(vec![SystemMessage {
            text: "You are a helpful assistant.".to_string(),
        }]);

        let (truncated_messages, truncated_system) = apply_truncation(&messages, &system, 2);

        // 应该保留最后 2 条消息 + 1 条截断提示
        assert_eq!(truncated_messages.len(), 3);
        assert_eq!(truncated_messages[0].role, "user");
        assert!(truncated_messages[0]
            .content
            .as_str()
            .unwrap()
            .contains("truncated"));

        // system 应该保留
        assert!(truncated_system.is_some());
    }

    #[test]
    fn test_apply_image_placeholder() {
        let messages = vec![
            Message {
                role: "user".to_string(),
                content: serde_json::json!([
                    {"type": "text", "text": "Look at this image:"},
                    {"type": "image", "source": {"type": "base64", "media_type": "image/png", "data": "base64data"}}
                ]),
            },
            Message {
                role: "assistant".to_string(),
                content: serde_json::json!("I see the image."),
            },
        ];

        let processed = apply_image_placeholder(&messages);

        // 检查图片是否被替换
        let first_msg_content = &processed[0].content;
        if let serde_json::Value::Array(arr) = first_msg_content {
            assert_eq!(arr.len(), 2);
            // 第二个元素应该是占位符
            assert_eq!(arr[1]["type"], "text");
            assert_eq!(arr[1]["text"], "[Image]");
        } else {
            panic!("Expected array content");
        }
    }

    #[test]
    fn test_manage_history_no_truncation() {
        let config = HistoryConfig {
            enabled: true,
            truncate_threshold: 1_000_000, // 很高的阈值
            enable_ai_summary: false,
            enable_image_placeholder: false,
            enable_prompt_caching: false,
            keep_recent_messages: 20,
        };

        let messages = vec![
            Message {
                role: "user".to_string(),
                content: serde_json::json!("Hello"),
            },
            Message {
                role: "assistant".to_string(),
                content: serde_json::json!("Hi there!"),
            },
        ];

        let result = manage_history(&config, messages.clone(), None, None);

        assert!(!result.truncated);
        assert!(!result.summarized);
        assert_eq!(result.messages.len(), messages.len());
    }

    #[test]
    fn test_manage_history_with_truncation() {
        let config = HistoryConfig {
            enabled: true,
            truncate_threshold: 5, // 非常低的阈值，强制截断
            enable_ai_summary: false,
            enable_image_placeholder: false,
            enable_prompt_caching: false,
            keep_recent_messages: 1,
        };

        let messages = vec![
            Message {
                role: "user".to_string(),
                content: serde_json::json!("This is a very long message that should definitely exceed the token threshold when combined with other messages in the conversation history"),
            },
            Message {
                role: "assistant".to_string(),
                content: serde_json::json!("This is another very long response that adds to the total token count and should help trigger the truncation mechanism"),
            },
            Message {
                role: "user".to_string(),
                content: serde_json::json!("And here is yet another message to make sure we have enough tokens"),
            },
        ];

        let result = manage_history(&config, messages, None, None);

        assert!(result.truncated);
        assert!(!result.summarized);
        // 应该有截断提示 + 保留的消息
        assert!(result.messages.len() <= 2);
    }

    #[test]
    fn test_estimate_message_tokens() {
        // 测试文本消息
        let text_msg = Message {
            role: "user".to_string(),
            content: serde_json::json!("Hello world"),
        };
        let tokens = estimate_message_tokens(&text_msg);
        assert!(tokens > 0);

        // 测试包含图片的消息
        let image_msg = Message {
            role: "user".to_string(),
            content: serde_json::json!([
                {"type": "text", "text": "Look at this:"},
                {"type": "image", "source": {"type": "base64", "media_type": "image/png", "data": "data"}}
            ]),
        };
        let tokens_with_image = estimate_message_tokens(&image_msg);
        assert!(tokens_with_image > 1000); // 图片估算为 1000 tokens
    }
}
