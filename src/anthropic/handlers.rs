//! Anthropic API Handler 函数
//!
//! HTTP 处理器，负责：
//! - 接收和解析 HTTP 请求
//! - 调用 service 层处理业务逻辑
//! - 构建和返回 HTTP 响应

use std::convert::Infallible;
use std::sync::Arc;

use crate::kiro::model::events::Event;
use crate::kiro::parser::decoder::EventStreamDecoder;
use crate::kiro::provider::KiroProvider;
use crate::token;
use axum::{
    Extension,
    Json as JsonExtractor,
    body::Body,
    extract::State,
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Json, Response},
};
use bytes::Bytes;
use futures::{Stream, StreamExt, stream};
use serde_json::json;
use std::time::Duration;
use tokio::time::interval;
use uuid::Uuid;

use super::converter::ConversionError;
use super::middleware::{AppState, AuthenticatedPoolId};
use super::service::{
    self, CONTEXT_WINDOW_SIZE, PING_INTERVAL_SECS, RequestContext, ValidationResult,
};
use super::stream::{BufferedStreamContext, SseEvent, StreamContext};
use super::types::{
    CountTokensRequest, CountTokensResponse, ErrorResponse, MessagesRequest, Model, ModelsResponse,
};
use super::websearch;

/// GET /v1/models
///
/// 返回可用的模型列表
pub async fn get_models() -> impl IntoResponse {
    tracing::info!("Received GET /v1/models request");

    let models = vec![
        Model {
            id: "claude-sonnet-4-5-20250929".to_string(),
            object: "model".to_string(),
            created: 1727568000,
            owned_by: "anthropic".to_string(),
            display_name: "Claude Sonnet 4.5".to_string(),
            model_type: "chat".to_string(),
            max_tokens: 32000,
        },
        Model {
            id: "claude-opus-4-5-20251101".to_string(),
            object: "model".to_string(),
            created: 1730419200,
            owned_by: "anthropic".to_string(),
            display_name: "Claude Opus 4.5".to_string(),
            model_type: "chat".to_string(),
            max_tokens: 32000,
        },
        Model {
            id: "claude-opus-4-6-20260206".to_string(),
            object: "model".to_string(),
            created: 1770314400,
            owned_by: "anthropic".to_string(),
            display_name: "Claude Opus 4.6".to_string(),
            model_type: "chat".to_string(),
            max_tokens: 32000,
        },
        Model {
            id: "claude-haiku-4-5-20251001".to_string(),
            object: "model".to_string(),
            created: 1727740800,
            owned_by: "anthropic".to_string(),
            display_name: "Claude Haiku 4.5".to_string(),
            model_type: "chat".to_string(),
            max_tokens: 32000,
        },
    ];

    Json(ModelsResponse {
        object: "list".to_string(),
        data: models,
    })
}

/// POST /v1/messages
///
/// 创建消息（对话）
pub async fn post_messages(
    State(state): State<AppState>,
    Extension(pool_id): Extension<AuthenticatedPoolId>,
    headers: HeaderMap,
    JsonExtractor(payload): JsonExtractor<MessagesRequest>,
) -> Response {
    handle_messages_request(state, pool_id, headers, payload, "/v1/messages", false).await
}

/// POST /cc/v1/messages
///
/// Claude Code 兼容端点，与 /v1/messages 的区别在于：
/// - 流式响应会等待 kiro 端返回 contextUsageEvent 后再发送 message_start
/// - message_start 中的 input_tokens 是从 contextUsageEvent 计算的准确值
pub async fn post_messages_cc(
    State(state): State<AppState>,
    Extension(pool_id): Extension<AuthenticatedPoolId>,
    headers: HeaderMap,
    JsonExtractor(payload): JsonExtractor<MessagesRequest>,
) -> Response {
    handle_messages_request(state, pool_id, headers, payload, "/cc/v1/messages", true).await
}

/// 处理消息请求的通用逻辑
///
/// # 参数
/// - `state`: 应用状态
/// - `pool_id`: 认证后的池 ID（来自 API Key 绑定）
/// - `headers`: HTTP 请求头
/// - `payload`: 消息请求体
/// - `endpoint`: 端点名称（用于日志）
/// - `use_buffered_stream`: 是否使用缓冲流（Claude Code 端点需要）
async fn handle_messages_request(
    state: AppState,
    pool_id: AuthenticatedPoolId,
    headers: HeaderMap,
    payload: MessagesRequest,
    endpoint: &str,
    use_buffered_stream: bool,
) -> Response {
    log_request(&payload, &headers, endpoint, &pool_id);

    // 根据 pool_id 选择 KiroProvider
    let kiro_provider = match resolve_kiro_provider(&state, &pool_id) {
        Ok(provider) => provider,
        Err(pool_error) => {
            return create_error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "pool_unavailable",
                &pool_error,
            );
        }
    };

    // 验证并准备请求
    match service::validate_and_prepare_request(
        kiro_provider.as_ref(),
        state.profile_arn.as_ref(),
        &payload,
        &headers,
        &state.config,
    ) {
        ValidationResult::Ok(ctx) => {
            handle_validated_request(ctx, use_buffered_stream).await
        }
        ValidationResult::ProviderNotConfigured => {
            create_error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "service_unavailable",
                "Kiro API provider not configured",
            )
        }
        ValidationResult::WebSearchRequest { provider, input_tokens } => {
            websearch::handle_websearch_request(provider, &payload, input_tokens).await
        }
        ValidationResult::ConversionFailed(e) => {
            create_conversion_error_response(e)
        }
        ValidationResult::SerializationFailed(msg) => {
            create_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error",
                &format!("序列化请求失败: {}", msg),
            )
        }
    }
}

/// 根据 pool_id 解析 KiroProvider
///
/// # 返回
/// - `Ok(Some(provider))` - 成功获取 Provider
/// - `Ok(None)` - 无 Provider 配置
/// - `Err(msg)` - API Key 绑定的池不可用（不应回退）
fn resolve_kiro_provider(
    state: &AppState,
    pool_id: &AuthenticatedPoolId,
) -> Result<Option<Arc<KiroProvider>>, String> {
    // 如果有 PoolManager，尝试根据 pool_id 获取池
    if let Some(ref pool_manager) = state.pool_manager {
        let pool_id_str = pool_id.0.as_deref();

        // 如果 API Key 绑定了特定池，必须使用该池
        if let Some(bound_pool_id) = pool_id_str {
            if let Some(pool_runtime) = pool_manager.get_pool_for_api_key(Some(bound_pool_id)) {
                tracing::debug!(
                    pool_id = ?pool_id_str,
                    actual_pool = %pool_runtime.config.id,
                    "使用 API Key 绑定的池"
                );
                // 为该池创建 KiroProvider
                let provider = KiroProvider::new(pool_runtime.token_manager.clone());
                return Ok(Some(Arc::new(provider)));
            } else {
                // API Key 绑定的池不可用，返回错误而不是回退
                tracing::error!(
                    pool_id = ?bound_pool_id,
                    "API Key 绑定的池不可用，拒绝请求"
                );
                return Err(format!(
                    "API Key 绑定的池 '{}' 不可用或已禁用",
                    bound_pool_id
                ));
            }
        }

        // API Key 未绑定特定池，使用默认池
        if let Some(pool_runtime) = pool_manager.get_pool_for_api_key(None) {
            tracing::debug!("使用默认池");
            let provider = KiroProvider::new(pool_runtime.token_manager.clone());
            return Ok(Some(Arc::new(provider)));
        }
    }

    // 回退到默认的 kiro_provider（无 PoolManager 时）
    Ok(state.kiro_provider.clone())
}

/// POST /v1/messages/count_tokens
///
/// 计算消息的 token 数量
pub async fn count_tokens(
    JsonExtractor(payload): JsonExtractor<CountTokensRequest>,
) -> impl IntoResponse {
    tracing::info!(
        model = %payload.model,
        message_count = %payload.messages.len(),
        "Received POST /v1/messages/count_tokens request"
    );

    let total_tokens = token::count_all_tokens(
        payload.model,
        payload.system,
        payload.messages,
        payload.tools,
    ) as i32;

    Json(CountTokensResponse {
        input_tokens: total_tokens.max(1) as i32,
    })
}

// ============ 内部辅助函数 ============

/// 记录请求日志
fn log_request(payload: &MessagesRequest, headers: &HeaderMap, endpoint: &str, pool_id: &AuthenticatedPoolId) {
    let session_id = service::extract_session_id(payload, headers);
    tracing::info!(
        model = %payload.model,
        max_tokens = %payload.max_tokens,
        stream = %payload.stream,
        message_count = %payload.messages.len(),
        session_id = ?session_id.as_ref().map(|s| &s[..s.len().min(30)]),
        pool_id = ?pool_id.0,
        "Received POST {} request", endpoint
    );
}

/// 创建错误响应
fn create_error_response(status: StatusCode, error_type: &str, message: &str) -> Response {
    (status, Json(ErrorResponse::new(error_type, message))).into_response()
}

/// 创建转换错误响应
fn create_conversion_error_response(e: ConversionError) -> Response {
    let (error_type, message) = match &e {
        ConversionError::UnsupportedModel(model) => {
            ("invalid_request_error", format!("模型不支持: {}", model))
        }
        ConversionError::EmptyMessages => {
            ("invalid_request_error", "消息列表为空".to_string())
        }
    };
    create_error_response(StatusCode::BAD_REQUEST, error_type, &message)
}

/// 处理已验证的请求
async fn handle_validated_request(ctx: RequestContext, use_buffered_stream: bool) -> Response {
    if ctx.is_stream {
        handle_stream_request(ctx, use_buffered_stream).await
    } else {
        handle_non_stream_request(ctx).await
    }
}

/// 处理流式请求
///
/// # 参数
/// - `ctx`: 请求上下文
/// - `use_buffered_stream`: 是否使用缓冲流模式
///   - `false`: 标准流模式，立即发送 message_start
///   - `true`: 缓冲流模式（Claude Code），等待 contextUsageEvent 后再发送
async fn handle_stream_request(ctx: RequestContext, use_buffered_stream: bool) -> Response {
    // Handler 层重试配置
    const MAX_HANDLER_RETRIES: usize = 2;
    let mut last_error = None;

    for attempt in 0..MAX_HANDLER_RETRIES {
        // 调用 Kiro API（支持粘性会话轮询 + 多凭据故障转移）
        let response = match ctx
            .provider
            .call_api_stream_with_session(&ctx.request_body, ctx.session_id.as_deref())
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                let error_msg = e.to_string();
                // 判断是否为可重试的错误（502/503/504 或网络错误）
                let is_retryable = error_msg.contains("502")
                    || error_msg.contains("503")
                    || error_msg.contains("504")
                    || error_msg.contains("Bad Gateway")
                    || error_msg.contains("Service Unavailable")
                    || error_msg.contains("Gateway Timeout")
                    || error_msg.contains("connection")
                    || error_msg.contains("timeout");

                if is_retryable && attempt + 1 < MAX_HANDLER_RETRIES {
                    tracing::warn!(
                        "Kiro API 调用失败（尝试 {}/{}），准备重试: {}",
                        attempt + 1,
                        MAX_HANDLER_RETRIES,
                        error_msg
                    );
                    last_error = Some(error_msg);
                    // 短暂延迟后重试
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                    continue;
                }

                tracing::error!("Kiro API 调用失败: {}", e);
                return create_error_response(
                    StatusCode::BAD_GATEWAY,
                    "api_error",
                    &format!("上游 API 调用失败: {}", e),
                );
            }
        };

        // 成功获取响应，根据模式创建不同的 SSE 流
        if use_buffered_stream {
            // 缓冲流模式：等待 contextUsageEvent 后再发送 message_start
            let buffered_ctx = BufferedStreamContext::new(
                &ctx.model,
                ctx.input_tokens,
                ctx.thinking_enabled,
            );
            let stream = create_buffered_sse_stream(response, buffered_ctx);
            return build_sse_response(stream);
        } else {
            // 标准流模式：立即发送 message_start
            let mut stream_ctx = StreamContext::new_with_thinking(
                &ctx.model,
                ctx.input_tokens,
                ctx.thinking_enabled,
            );
            let initial_events = stream_ctx.generate_initial_events();
            let stream = create_sse_stream(response, stream_ctx, initial_events);
            return build_sse_response(stream);
        }
    }

    // 所有重试都失败
    create_error_response(
        StatusCode::BAD_GATEWAY,
        "api_error",
        &format!(
            "上游 API 调用失败（已重试 {} 次）: {}",
            MAX_HANDLER_RETRIES,
            last_error.unwrap_or_else(|| "未知错误".to_string())
        ),
    )
}

/// 处理非流式请求
async fn handle_non_stream_request(ctx: RequestContext) -> Response {
    // Handler 层重试配置
    const MAX_HANDLER_RETRIES: usize = 2;
    let mut last_error = None;

    for attempt in 0..MAX_HANDLER_RETRIES {
        // 调用 Kiro API（支持粘性会话轮询 + 多凭据故障转移）
        let response = match ctx
            .provider
            .call_api_with_session(&ctx.request_body, ctx.session_id.as_deref())
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                let error_msg = e.to_string();
                // 判断是否为可重试的错误（502/503/504 或网络错误）
                let is_retryable = error_msg.contains("502")
                    || error_msg.contains("503")
                    || error_msg.contains("504")
                    || error_msg.contains("Bad Gateway")
                    || error_msg.contains("Service Unavailable")
                    || error_msg.contains("Gateway Timeout")
                    || error_msg.contains("connection")
                    || error_msg.contains("timeout");

                if is_retryable && attempt + 1 < MAX_HANDLER_RETRIES {
                    tracing::warn!(
                        "Kiro API 调用失败（尝试 {}/{}），准备重试: {}",
                        attempt + 1,
                        MAX_HANDLER_RETRIES,
                        error_msg
                    );
                    last_error = Some(error_msg);
                    // 短暂延迟后重试
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                    continue;
                }

                tracing::error!("Kiro API 调用失败: {}", e);
                return create_error_response(
                    StatusCode::BAD_GATEWAY,
                    "api_error",
                    &format!("上游 API 调用失败: {}", e),
                );
            }
        };

        // 读取响应体
        let body_bytes = match response.bytes().await {
            Ok(bytes) => bytes,
            Err(e) => {
                let error_msg = e.to_string();
                if attempt + 1 < MAX_HANDLER_RETRIES {
                    tracing::warn!(
                        "读取响应体失败（尝试 {}/{}），准备重试: {}",
                        attempt + 1,
                        MAX_HANDLER_RETRIES,
                        error_msg
                    );
                    last_error = Some(error_msg);
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                    continue;
                }

                tracing::error!("读取响应体失败: {}", e);
                return create_error_response(
                    StatusCode::BAD_GATEWAY,
                    "api_error",
                    &format!("读取响应失败: {}", e),
                );
            }
        };

        // 解析事件流并构建响应
        return build_non_stream_response(&body_bytes, &ctx.model, ctx.input_tokens);
    }

    // 所有重试都失败
    create_error_response(
        StatusCode::BAD_GATEWAY,
        "api_error",
        &format!(
            "上游 API 调用失败（已重试 {} 次）: {}",
            MAX_HANDLER_RETRIES,
            last_error.unwrap_or_else(|| "未知错误".to_string())
        ),
    )
}

/// 构建非流式响应
fn build_non_stream_response(body_bytes: &[u8], model: &str, input_tokens: i32) -> Response {
    // 解析事件流
    let mut decoder = EventStreamDecoder::new();
    if let Err(e) = decoder.feed(body_bytes) {
        tracing::warn!("缓冲区溢出: {}", e);
    }

    let mut text_content = String::new();
    let mut tool_uses: Vec<serde_json::Value> = Vec::new();
    let mut has_tool_use = false;
    let mut stop_reason = "end_turn".to_string();
    let mut context_input_tokens: Option<i32> = None;
    let mut tool_json_buffers: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();

    for result in decoder.decode_iter() {
        match result {
            Ok(frame) => {
                if let Ok(event) = Event::from_frame(frame) {
                    match event {
                        Event::AssistantResponse(resp) => {
                            text_content.push_str(&resp.content);
                        }
                        Event::ToolUse(tool_use) => {
                            has_tool_use = true;
                            let buffer = tool_json_buffers
                                .entry(tool_use.tool_use_id.clone())
                                .or_insert_with(String::new);
                            buffer.push_str(&tool_use.input);

                            if tool_use.stop {
                                let input: serde_json::Value = serde_json::from_str(buffer)
                                    .unwrap_or_else(|e| {
                                        tracing::warn!(
                                            "工具输入 JSON 解析失败: {}, tool_use_id: {}, 原始内容: {}",
                                            e, tool_use.tool_use_id, buffer
                                        );
                                        serde_json::json!({})
                                    });

                                tool_uses.push(json!({
                                    "type": "tool_use",
                                    "id": tool_use.tool_use_id,
                                    "name": tool_use.name,
                                    "input": input
                                }));
                            }
                        }
                        Event::ContextUsage(context_usage) => {
                            let actual_input_tokens = (context_usage.context_usage_percentage
                                * (CONTEXT_WINDOW_SIZE as f64)
                                / 100.0) as i32;
                            context_input_tokens = Some(actual_input_tokens);
                            tracing::debug!(
                                "收到 contextUsageEvent: {}%, 计算 input_tokens: {}",
                                context_usage.context_usage_percentage,
                                actual_input_tokens
                            );
                        }
                        Event::Exception { exception_type, .. } => {
                            if exception_type == "ContentLengthExceededException" {
                                stop_reason = "max_tokens".to_string();
                            }
                        }
                        _ => {}
                    }
                }
            }
            Err(e) => {
                tracing::warn!("解码事件失败: {}", e);
            }
        }
    }

    // 确定 stop_reason
    if has_tool_use && stop_reason == "end_turn" {
        stop_reason = "tool_use".to_string();
    }

    // 构建响应内容
    let mut content: Vec<serde_json::Value> = Vec::new();
    if !text_content.is_empty() {
        content.push(json!({
            "type": "text",
            "text": text_content
        }));
    }
    content.extend(tool_uses);

    // 估算输出 tokens
    let output_tokens = token::estimate_output_tokens(&content);
    let final_input_tokens = context_input_tokens.unwrap_or(input_tokens);

    // 构建 Anthropic 响应
    let response_body = json!({
        "id": format!("msg_{}", Uuid::new_v4().to_string().replace('-', "")),
        "type": "message",
        "role": "assistant",
        "content": content,
        "model": model,
        "stop_reason": stop_reason,
        "stop_sequence": null,
        "usage": {
            "input_tokens": final_input_tokens,
            "output_tokens": output_tokens
        }
    });

    (StatusCode::OK, Json(response_body)).into_response()
}

/// 构建 SSE 响应
fn build_sse_response<S>(stream: S) -> Response
where
    S: Stream<Item = Result<Bytes, Infallible>> + Send + 'static,
{
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/event-stream")
        .header(header::CACHE_CONTROL, "no-cache")
        .header(header::CONNECTION, "keep-alive")
        .body(Body::from_stream(stream))
        .unwrap()
}

/// 创建 ping 事件的 SSE 字符串
fn create_ping_sse() -> Bytes {
    Bytes::from("event: ping\ndata: {\"type\": \"ping\"}\n\n")
}

/// 创建 SSE 事件流
fn create_sse_stream(
    response: reqwest::Response,
    ctx: StreamContext,
    initial_events: Vec<SseEvent>,
) -> impl Stream<Item = Result<Bytes, Infallible>> {
    // 先发送初始事件
    let initial_stream = stream::iter(
        initial_events
            .into_iter()
            .map(|e| Ok(Bytes::from(e.to_sse_string()))),
    );

    // 然后处理 Kiro 响应流，同时每25秒发送 ping 保活
    let body_stream = response.bytes_stream();

    let processing_stream = stream::unfold(
        (body_stream, ctx, EventStreamDecoder::new(), false, interval(Duration::from_secs(PING_INTERVAL_SECS))),
        |(mut body_stream, mut ctx, mut decoder, finished, mut ping_interval)| async move {
            if finished {
                return None;
            }

            tokio::select! {
                chunk_result = body_stream.next() => {
                    match chunk_result {
                        Some(Ok(chunk)) => {
                            if let Err(e) = decoder.feed(&chunk) {
                                tracing::warn!("缓冲区溢出: {}", e);
                            }

                            let mut events = Vec::new();
                            for result in decoder.decode_iter() {
                                match result {
                                    Ok(frame) => {
                                        if let Ok(event) = Event::from_frame(frame) {
                                            let sse_events = ctx.process_kiro_event(&event);
                                            events.extend(sse_events);
                                        }
                                    }
                                    Err(e) => {
                                        tracing::warn!("解码事件失败: {}", e);
                                    }
                                }
                            }

                            let bytes: Vec<Result<Bytes, Infallible>> = events
                                .into_iter()
                                .map(|e| Ok(Bytes::from(e.to_sse_string())))
                                .collect();

                            Some((stream::iter(bytes), (body_stream, ctx, decoder, false, ping_interval)))
                        }
                        Some(Err(e)) => {
                            tracing::error!("读取响应流失败: {}", e);
                            let final_events = ctx.generate_final_events();
                            let bytes: Vec<Result<Bytes, Infallible>> = final_events
                                .into_iter()
                                .map(|e| Ok(Bytes::from(e.to_sse_string())))
                                .collect();
                            Some((stream::iter(bytes), (body_stream, ctx, decoder, true, ping_interval)))
                        }
                        None => {
                            let final_events = ctx.generate_final_events();
                            let bytes: Vec<Result<Bytes, Infallible>> = final_events
                                .into_iter()
                                .map(|e| Ok(Bytes::from(e.to_sse_string())))
                                .collect();
                            Some((stream::iter(bytes), (body_stream, ctx, decoder, true, ping_interval)))
                        }
                    }
                }
                _ = ping_interval.tick() => {
                    tracing::trace!("发送 ping 保活事件");
                    let bytes: Vec<Result<Bytes, Infallible>> = vec![Ok(create_ping_sse())];
                    Some((stream::iter(bytes), (body_stream, ctx, decoder, false, ping_interval)))
                }
            }
        },
    )
    .flatten();

    initial_stream.chain(processing_stream)
}

/// 创建缓冲 SSE 事件流
fn create_buffered_sse_stream(
    response: reqwest::Response,
    ctx: BufferedStreamContext,
) -> impl Stream<Item = Result<Bytes, Infallible>> {
    let body_stream = response.bytes_stream();

    stream::unfold(
        (
            body_stream,
            ctx,
            EventStreamDecoder::new(),
            false,
            interval(Duration::from_secs(PING_INTERVAL_SECS)),
        ),
        |(mut body_stream, mut ctx, mut decoder, finished, mut ping_interval)| async move {
            if finished {
                return None;
            }

            loop {
                tokio::select! {
                    biased;

                    _ = ping_interval.tick() => {
                        tracing::trace!("发送 ping 保活事件（缓冲模式）");
                        let bytes: Vec<Result<Bytes, Infallible>> = vec![Ok(create_ping_sse())];
                        return Some((stream::iter(bytes), (body_stream, ctx, decoder, false, ping_interval)));
                    }

                    chunk_result = body_stream.next() => {
                        match chunk_result {
                            Some(Ok(chunk)) => {
                                if let Err(e) = decoder.feed(&chunk) {
                                    tracing::warn!("缓冲区溢出: {}", e);
                                }

                                for result in decoder.decode_iter() {
                                    match result {
                                        Ok(frame) => {
                                            if let Ok(event) = Event::from_frame(frame) {
                                                ctx.process_and_buffer(&event);
                                            }
                                        }
                                        Err(e) => {
                                            tracing::warn!("解码事件失败: {}", e);
                                        }
                                    }
                                }
                            }
                            Some(Err(e)) => {
                                tracing::error!("读取响应流失败: {}", e);
                                let all_events = ctx.finish_and_get_all_events();
                                let bytes: Vec<Result<Bytes, Infallible>> = all_events
                                    .into_iter()
                                    .map(|e| Ok(Bytes::from(e.to_sse_string())))
                                    .collect();
                                return Some((stream::iter(bytes), (body_stream, ctx, decoder, true, ping_interval)));
                            }
                            None => {
                                let all_events = ctx.finish_and_get_all_events();
                                let bytes: Vec<Result<Bytes, Infallible>> = all_events
                                    .into_iter()
                                    .map(|e| Ok(Bytes::from(e.to_sse_string())))
                                    .collect();
                                return Some((stream::iter(bytes), (body_stream, ctx, decoder, true, ping_interval)));
                            }
                        }
                    }
                }
            }
        },
    )
    .flatten()
}
