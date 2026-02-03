# Kiro.rs CLI 工具

命令行工具，用于管理凭据、扫描 Token、生成登录链接等。

## 安装

```bash
# 编译 CLI 工具
cargo build --release --bin kiro-cli

# 安装到系统路径（可选）
cargo install --path . --bin kiro-cli
```

编译后的二进制文件位于 `target/release/kiro-cli`（Windows 上为 `kiro-cli.exe`）。

## 使用方法

### 凭据管理

#### 列出所有凭据

```bash
kiro-cli credentials list
kiro-cli credentials list --file config/credentials.json
```

显示所有凭据的详细信息，包括：
- ID、认证方式、优先级
- Region、池 ID
- 过期时间、Profile ARN
- 调用统计（成功/失败次数、最后调用时间）

#### 添加新凭据

```bash
# Social 认证
kiro-cli credentials add \
  --refresh-token "YOUR_REFRESH_TOKEN" \
  --auth-method social \
  --priority 0 \
  --region us-east-1

# IdC 认证
kiro-cli credentials add \
  --refresh-token "YOUR_REFRESH_TOKEN" \
  --auth-method idc \
  --priority 0 \
  --region us-east-1 \
  --client-id "YOUR_CLIENT_ID" \
  --client-secret "YOUR_CLIENT_SECRET"
```

参数说明：
- `--refresh-token`: Refresh Token（必需）
- `--auth-method`: 认证方式，`social` 或 `idc`（默认：`social`）
- `--priority`: 优先级，数字越小优先级越高（默认：`0`）
- `--region`: AWS Region（可选）
- `--client-id`: IdC Client ID（IdC 认证需要）
- `--client-secret`: IdC Client Secret（IdC 认证需要）
- `--file`: 凭据文件路径（默认：`config/credentials.json`）

#### 删除凭据

```bash
kiro-cli credentials delete --id 1
kiro-cli credentials delete --id 1 --file config/credentials.json
```

#### 导入凭据

```bash
# 从 JSON 文件导入
kiro-cli credentials import \
  --input backup.json \
  --output config/credentials.json \
  --format json

# 从 YAML 文件导入
kiro-cli credentials import \
  --input backup.yaml \
  --output config/credentials.json \
  --format yaml
```

导入功能会：
- 自动合并到现有凭据
- 为新凭据分配唯一 ID
- 避免 ID 冲突

#### 导出凭据

```bash
# 导出为 JSON
kiro-cli credentials export \
  --input config/credentials.json \
  --output backup.json \
  --format json

# 导出为 YAML
kiro-cli credentials export \
  --input config/credentials.json \
  --output backup.yaml \
  --format yaml
```

### Token 扫描和验证

#### 扫描本地 Token

```bash
kiro-cli token scan
kiro-cli token scan --file config/credentials.json
```

扫描功能会显示：
- Token 类型（Refresh Token / Access Token）
- Token 长度和预览
- 过期时间和状态
- Token 是否被截断
- IdC 配置完整性

#### 验证 Token 有效性

```bash
# 验证所有凭据
kiro-cli token validate \
  --file config/credentials.json \
  --config config/config.json

# 验证指定凭据
kiro-cli token validate \
  --file config/credentials.json \
  --config config/config.json \
  --id 1
```

验证功能会检查：
- Refresh Token 是否存在
- Token 长度是否足够
- Token 是否被截断
- 过期状态
- IdC 认证配置完整性

#### 刷新 Token

```bash
# 刷新所有凭据
kiro-cli token refresh \
  --file config/credentials.json \
  --config config/config.json

# 刷新指定凭据
kiro-cli token refresh \
  --file config/credentials.json \
  --config config/config.json \
  --id 1
```

刷新功能会：
- 调用 AWS 认证服务刷新 Token
- 更新 Access Token 和过期时间
- 自动保存更新后的凭据
- 显示刷新结果统计

### OAuth 登录链接生成

#### 生成 Social 认证登录链接

```bash
kiro-cli auth login \
  --auth-method social \
  --region us-east-1
```

输出：
- OAuth 授权 URL
- 登录步骤说明
- 添加凭据的命令示例
- 注意事项

#### 生成 IdC 认证登录链接

```bash
kiro-cli auth login \
  --auth-method idc \
  --region us-east-1 \
  --client-id "YOUR_CLIENT_ID"
```

输出：
- OIDC 授权 URL
- 登录步骤说明
- 回调 URL 处理方法
- 添加凭据的命令示例

## 配置文件

### 凭据文件格式（credentials.json）

```json
[
  {
    "id": 1,
    "refreshToken": "YOUR_REFRESH_TOKEN",
    "authMethod": "social",
    "priority": 0,
    "region": "us-east-1",
    "successCount": 100,
    "totalFailureCount": 2,
    "lastCallTime": 1704067200000
  },
  {
    "id": 2,
    "refreshToken": "YOUR_REFRESH_TOKEN",
    "authMethod": "idc",
    "priority": 1,
    "region": "us-west-2",
    "clientId": "YOUR_CLIENT_ID",
    "clientSecret": "YOUR_CLIENT_SECRET"
  }
]
```

### 配置文件格式（config.json）

```json
{
  "host": "127.0.0.1",
  "port": 8080,
  "region": "us-east-1",
  "kiroVersion": "0.8.0",
  "proxyUrl": "socks5://127.0.0.1:1080",
  "proxyUsername": "user",
  "proxyPassword": "pass"
}
```

## 使用场景

### 场景 1：批量导入凭据

```bash
# 1. 准备凭据文件（JSON 或 YAML）
cat > import.json << EOF
[
  {
    "refreshToken": "token1",
    "authMethod": "social",
    "priority": 0
  },
  {
    "refreshToken": "token2",
    "authMethod": "social",
    "priority": 1
  }
]
EOF

# 2. 导入凭据
kiro-cli credentials import --input import.json --output config/credentials.json

# 3. 验证导入结果
kiro-cli credentials list
```

### 场景 2：定期刷新 Token

```bash
# 创建定时任务脚本
cat > refresh_tokens.sh << 'EOF'
#!/bin/bash
echo "开始刷新 Token..."
kiro-cli token refresh \
  --file config/credentials.json \
  --config config/config.json
echo "刷新完成"
EOF

chmod +x refresh_tokens.sh

# 添加到 crontab（每天凌晨 2 点执行）
# 0 2 * * * /path/to/refresh_tokens.sh >> /var/log/kiro-refresh.log 2>&1
```

### 场景 3：备份和恢复凭据

```bash
# 备份凭据
kiro-cli credentials export \
  --input config/credentials.json \
  --output backup/credentials-$(date +%Y%m%d).json

# 恢复凭据
kiro-cli credentials import \
  --input backup/credentials-20260204.json \
  --output config/credentials.json
```

### 场景 4：健康检查

```bash
# 扫描所有 Token 状态
kiro-cli token scan --file config/credentials.json

# 验证 Token 有效性
kiro-cli token validate \
  --file config/credentials.json \
  --config config/config.json

# 刷新即将过期的 Token
kiro-cli token refresh \
  --file config/credentials.json \
  --config config/config.json
```

## 自动化集成

### CI/CD 集成

```yaml
# .github/workflows/refresh-tokens.yml
name: Refresh Tokens

on:
  schedule:
    - cron: '0 2 * * *'  # 每天凌晨 2 点
  workflow_dispatch:

jobs:
  refresh:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Setup Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      - name: Build CLI
        run: cargo build --release --bin kiro-cli

      - name: Refresh Tokens
        run: |
          ./target/release/kiro-cli token refresh \
            --file config/credentials.json \
            --config config/config.json

      - name: Commit Changes
        run: |
          git config user.name "GitHub Actions"
          git config user.email "actions@github.com"
          git add config/credentials.json
          git commit -m "chore: refresh tokens" || true
          git push
```

### Docker 集成

```dockerfile
# Dockerfile.cli
FROM rust:1.93 as builder

WORKDIR /app
COPY . .
RUN cargo build --release --bin kiro-cli

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/kiro-cli /usr/local/bin/
ENTRYPOINT ["kiro-cli"]
```

使用：

```bash
# 构建镜像
docker build -f Dockerfile.cli -t kiro-cli .

# 运行命令
docker run --rm -v $(pwd)/config:/config kiro-cli credentials list --file /config/credentials.json
```

## 故障排查

### Token 被截断

如果看到 "Token 可能已被截断" 警告：

1. 确保从 Kiro IDE 复制完整的 Token
2. Token 长度通常 > 200 字符
3. 不要使用文本编辑器打开凭据文件（可能自动换行）

### 刷新失败

如果 Token 刷新失败：

1. 检查网络连接和代理配置
2. 验证 Region 配置是否正确
3. 对于 IdC 认证，确保 clientId 和 clientSecret 正确
4. 检查 Refresh Token 是否已过期或被撤销

### 导入失败

如果导入凭据失败：

1. 检查文件格式是否正确（JSON 或 YAML）
2. 确保文件编码为 UTF-8
3. 验证 JSON/YAML 语法是否正确

## 最佳实践

1. **定期备份凭据**：使用 `export` 命令定期备份凭据文件
2. **自动刷新 Token**：设置定时任务自动刷新 Token
3. **监控 Token 状态**：定期运行 `scan` 和 `validate` 检查 Token 健康状态
4. **使用优先级**：为不同凭据设置优先级，确保高质量凭据优先使用
5. **安全存储**：凭据文件包含敏感信息，确保适当的文件权限（如 `chmod 600`）

## 环境变量

CLI 工具支持以下环境变量：

- `RUST_LOG`: 日志级别（`error`, `warn`, `info`, `debug`, `trace`）

示例：

```bash
RUST_LOG=debug kiro-cli token refresh --file config/credentials.json --config config/config.json
```

## 许可证

与主项目相同。
