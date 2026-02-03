# 智能消息历史管理实施文档

## 概述

实现了智能消息历史管理功能，参考 KiroProxy 的四层策略，应对 Anthropic API 的长度限制，提升长对话场景的稳定性。

## 实施日期

2026-02-04

## 功能特性

### 四层策略

#### 策略 1：自动截断
- **触发条件**：消息历史超过配置的 token 阈值（默认 100k tokens）
- **处理方式**：
  - 保留最近的 N 条消息（默认 20 条）
  - 保留完整的 system prompt
  - 在截断位置添加提示消息：`[Earlier messages truncated to manage context length]`
- **优点**：简单高效，无需额外 API 调用
- **适用场景**：大多数长对话场景

#### 策略 2：AI 摘要
- **触发条件**：启用 AI 摘要配置且超过 token 阈值
- **处理方式**：使用 Haiku 模型摘要历史消息，将长历史压缩为简短摘要
- **状态**：预留接口，当前未实现（需要额外 API 调用）
- **优点**：保留更多上下文信息
- **适用场景**：需要保留完整对话上下文的场景

#### 策略 3：图片占位符
- **触发条件**：启用图片占位符配置（默认启用）
- **处理方式**：将历史消息中的图片替换为 `[Image]` 文本占位符
- **优点**：大幅减少 token 消耗（图片估算为 1000 tokens）
- **适用场景**：包含大量图片的对话

#### 策略 4：缓存复用
- **触发条件**：启用 Prompt Caching 配置
- **处理方式**：利用 Anthropic 的 Prompt Caching 功能缓存 system prompt 和历史消息
- **状态**：预留接口，当前未实现（需要 Kiro API 支持）
- **优点**：减少重复计算，提升响应速度
- **适用场景**：频繁使用相同 system prompt 的场景

## 实施细节

### 新增文件

#### 1. `src/anthropic/history.rs`
核心历史管理模块，包含：
- `HistoryConfig`: 历史管理配置结构
- `HistoryManagementResult`: 处理结果结构
- `manage_history()`: 主处理函数
- `apply_truncation()`: 自动截断实现
- `apply_ai_summary()`: AI 摘要占位符
- `apply_image_placeholder()`: 图片占位符实现
- `estimate_total_tokens()`: Token 估算函数

**关键代码**：
```rust
pub fn manage_history(
    config: &HistoryConfig,
    messages: Vec<Message>,
    system: Option<Vec<SystemMessage>>,
    tools: Option<&Vec<Tool>>,
) -> HistoryManagementResult
```

### 修改文件

#### 1. `src/anthropic/mod.rs`
- 添加 `history` 模块声明

#### 2. `src/anthropic/service.rs`
- 导入 `history` 模块
- 新增 `apply_history_management()` 函数
- 修改 `convert_and_build_request()` 函数，集成历史管理
- 修改 `validate_and_prepare_request()` 函数，传递配置参数

**关键代码**：
```rust
fn apply_history_management(
    payload: &MessagesRequest,
    config: &Config,
) -> MessagesRequest {
    let history_config = HistoryConfig {
        enabled: config.history_management_enabled,
        truncate_threshold: config.history_truncate_threshold,
        enable_ai_summary: config.history_enable_ai_summary,
        enable_image_placeholder: config.history_enable_image_placeholder,
        enable_prompt_caching: false,
        keep_recent_messages: config.history_keep_recent_messages,
    };

    let result = manage_history(
        &history_config,
        payload.messages.clone(),
        payload.system.clone(),
        payload.tools.as_ref(),
    );

    // 返回处理后的请求
    MessagesRequest { /* ... */ }
}
```

#### 3. `src/anthropic/middleware.rs`
- 添加 `Config` 导入
- 修改 `AppState` 结构，添加 `config` 字段
- 修改 `AppState::new()` 方法，接受 `config` 参数

**关键代码**：
```rust
pub struct AppState {
    pub kiro_provider: Option<Arc<KiroProvider>>,
    pub profile_arn: Option<String>,
    pub api_key_manager: Arc<ApiKeyManager>,
    pub pool_manager: Option<Arc<PoolManager>>,
    pub rate_limiter: Option<Arc<RateLimiter>>,
    pub config: Arc<Config>,  // 新增
}
```

#### 4. `src/anthropic/router.rs`
- 修改 `create_router()` 函数，传递 `config` 到 `AppState`

#### 5. `src/anthropic/handlers.rs`
- 修改 `validate_and_prepare_request()` 调用，传递 `config` 参数

#### 6. `src/model/config.rs`
- 添加历史管理配置字段：
  - `history_management_enabled`: 是否启用（默认 true）
  - `history_truncate_threshold`: 截断阈值（默认 100000 tokens）
  - `history_enable_ai_summary`: 是否启用 AI 摘要（默认 false）
  - `history_enable_image_placeholder`: 是否启用图片占位符（默认 true）
  - `history_keep_recent_messages`: 保留最近消息数量（默认 20）
- 添加对应的默认值函数
- 更新 `Default` 实现
- 更新 `validate()` 方法，添加配置验证

**配置示例**：
```json
{
  "historyManagementEnabled": true,
  "historyTruncateThreshold": 100000,
  "historyEnableAiSummary": false,
  "historyEnableImagePlaceholder": true,
  "historyKeepRecentMessages": 20
}
```

## 配置说明

### 配置项

| 配置项 | 类型 | 默认值 | 说明 |
|--------|------|--------|------|
| `historyManagementEnabled` | boolean | true | 是否启用智能历史管理 |
| `historyTruncateThreshold` | number | 100000 | 自动截断阈值（tokens） |
| `historyEnableAiSummary` | boolean | false | 是否启用 AI 摘要（需要额外 API 调用） |
| `historyEnableImagePlaceholder` | boolean | true | 是否启用图片占位符 |
| `historyKeepRecentMessages` | number | 20 | 截断时保留最近的消息数量 |

### 配置文件示例

```json
{
  "host": "127.0.0.1",
  "port": 8080,
  "historyManagementEnabled": true,
  "historyTruncateThreshold": 100000,
  "historyEnableAiSummary": false,
  "historyEnableImagePlaceholder": true,
  "historyKeepRecentMessages": 20
}
```

## 工作流程

1. **请求接收**：用户发送消息请求到 `/v1/messages` 或 `/cc/v1/messages`
2. **历史管理检查**：
   - 检查 `historyManagementEnabled` 配置
   - 如果禁用，直接跳过历史管理
3. **图片占位符**（如果启用）：
   - 遍历所有消息
   - 将 `type: "image"` 的内容块替换为 `type: "text", text: "[Image]"`
4. **Token 估算**：
   - 估算处理后的总 token 数量
   - 包括 system prompt、messages、tools
5. **阈值检查**：
   - 如果 tokens <= threshold，直接返回
   - 如果 tokens > threshold，应用截断或摘要策略
6. **策略应用**：
   - 如果启用 AI 摘要：调用 AI 摘要（当前回退到截断）
   - 否则：应用自动截断
7. **结果返回**：
   - 返回处理后的消息列表
   - 记录处理统计信息

## Token 估算

### 估算规则

- **文本消息**：使用 `token::count_tokens()` 精确计算
- **图片**：估算为 1000 tokens
- **tool_use**：input JSON 字符串 + 50 tokens 开销
- **tool_result**：content 字符串 + 50 tokens 开销
- **system prompt**：精确计算每个 SystemMessage
- **tools**：name + description + input_schema JSON

### 计算函数

```rust
fn estimate_total_tokens(
    messages: &[Message],
    system: &Option<Vec<SystemMessage>>,
    tools: Option<&Vec<Tool>>,
) -> u64
```

## 测试

### 测试用例

1. **test_apply_truncation**：测试自动截断功能
2. **test_apply_image_placeholder**：测试图片占位符替换
3. **test_manage_history_no_truncation**：测试未超过阈值的情况
4. **test_manage_history_with_truncation**：测试超过阈值触发截断
5. **test_estimate_message_tokens**：测试 token 估算准确性

### 运行测试

```bash
cargo test --bin kiro-rs history
```

### 测试结果

```
running 9 tests
test anthropic::history::tests::test_apply_truncation ... ok
test anthropic::history::tests::test_apply_image_placeholder ... ok
test anthropic::history::tests::test_estimate_message_tokens ... ok
test anthropic::history::tests::test_manage_history_no_truncation ... ok
test anthropic::history::tests::test_manage_history_with_truncation ... ok
test anthropic::converter::tests::test_collect_history_tool_names ... ok
test anthropic::converter::tests::test_history_tools_added_to_tools_list ... ok
test anthropic::converter::tests::test_validate_tool_pairing_history_already_paired ... ok
test kiro::model::requests::conversation::tests::test_history_serialize ... ok

test result: ok. 9 passed; 0 failed; 0 ignored
```

## 性能影响

### Token 减少效果

- **图片占位符**：每张图片节省约 900+ tokens
- **自动截断**：根据保留消息数量，可节省 50%-90% tokens
- **AI 摘要**（未实现）：预计可节省 60%-80% tokens 同时保留更多上下文

### 处理开销

- **图片占位符**：O(n) 遍历，开销极小
- **自动截断**：O(n) 遍历 + 数组切片，开销极小
- **Token 估算**：使用缓存的 tokenizer，开销可控

## 日志记录

### 日志级别

- **INFO**：应用历史管理策略时记录
- **DEBUG**：详细的处理步骤和 token 统计
- **WARN**：AI 摘要回退到截断时警告

### 日志示例

```
[INFO] 历史管理应用：truncated=true, summarized=false, image_placeholder=true, tokens: 150000 -> 45000
[INFO] 应用自动截断策略（tokens: 150000 > 100000）
[DEBUG] 截断历史消息：50 -> 20 条
[INFO] 历史管理完成：150000 tokens -> 45000 tokens (减少 70.0%)
```

## 未来改进

### 短期（1-2 周）

1. **实现 AI 摘要功能**
   - 集成 Haiku 模型调用
   - 实现摘要生成和插入逻辑
   - 添加摘要质量评估

2. **优化 Token 估算**
   - 使用更精确的 tokenizer
   - 缓存常见内容的 token 数量
   - 支持批量估算

### 中期（1-2 月）

1. **实现 Prompt Caching**
   - 研究 Anthropic Prompt Caching API
   - 实现缓存标记添加
   - 验证缓存效果

2. **添加监控指标**
   - 记录截断/摘要频率
   - 统计 token 节省效果
   - 监控处理延迟

### 长期（3-6 月）

1. **智能策略选择**
   - 根据对话类型自动选择策略
   - 机器学习优化阈值
   - 自适应调整保留消息数量

2. **分布式缓存**
   - 支持 Redis 缓存
   - 跨实例共享摘要
   - 提升缓存命中率

## 验证方法

### 功能验证

1. **发送超长对话请求**（> 100k tokens）
   ```bash
   curl -X POST http://localhost:8080/v1/messages \
     -H "Content-Type: application/json" \
     -H "x-api-key: YOUR_API_KEY" \
     -d '{
       "model": "claude-sonnet-4-5-20250929",
       "max_tokens": 1024,
       "messages": [
         // ... 大量历史消息 ...
       ]
     }'
   ```

2. **验证自动截断**
   - 检查响应是否成功
   - 查看日志确认截断应用
   - 验证保留的消息数量

3. **验证图片占位符**
   - 发送包含图片的消息
   - 检查历史消息中图片是否被替换
   - 验证 token 数量减少

4. **验证配置生效**
   - 修改配置文件
   - 重启服务
   - 验证新配置生效

### 性能验证

1. **Token 减少效果**
   - 记录原始 token 数量
   - 记录处理后 token 数量
   - 计算减少百分比

2. **响应时间**
   - 测量处理延迟
   - 对比启用/禁用历史管理的差异
   - 确保延迟在可接受范围内

3. **内存使用**
   - 监控内存占用
   - 验证无内存泄漏
   - 确保缓存大小合理

## 故障排查

### 常见问题

1. **历史管理未生效**
   - 检查 `historyManagementEnabled` 配置
   - 验证 token 数量是否超过阈值
   - 查看日志确认处理流程

2. **截断过于激进**
   - 调整 `historyTruncateThreshold` 阈值
   - 增加 `historyKeepRecentMessages` 数量
   - 考虑启用 AI 摘要

3. **图片占位符影响体验**
   - 禁用 `historyEnableImagePlaceholder`
   - 或调整阈值，仅在必要时应用

### 调试技巧

1. **启用详细日志**
   ```bash
   RUST_LOG=debug cargo run
   ```

2. **检查处理结果**
   - 查看 `HistoryManagementResult` 结构
   - 验证 `truncated`、`summarized` 标志
   - 检查 token 数量变化

3. **单元测试**
   ```bash
   cargo test --bin kiro-rs history -- --nocapture
   ```

## 总结

智能消息历史管理功能已成功实施，提供了四层策略应对长对话场景：

1. **自动截断**：已实现，默认启用
2. **AI 摘要**：接口预留，待实现
3. **图片占位符**：已实现，默认启用
4. **缓存复用**：接口预留，待实现

该功能通过配置文件灵活控制，支持动态调整策略，有效降低 token 消耗，提升长对话场景的稳定性和用户体验。

## 相关文档

- [Token 计算精准性提升](./token_accuracy_implementation.md)
- [会话粘性机制](./session_stickiness_implementation.md)
- [配置文件说明](../config/README.md)

## 变更历史

| 日期 | 版本 | 变更内容 |
|------|------|----------|
| 2026-02-04 | 1.0.0 | 初始实现，支持自动截断和图片占位符 |
