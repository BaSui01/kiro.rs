# 配置文件目录

此目录用于存放 kiro-rs 的配置文件，Docker 部署时会挂载到容器内。

## 文件说明

| 文件 | 必需 | 说明 |
|------|------|------|
| `config.json` | ✅ | 主配置文件 |
| `credentials.json` | ✅ | 凭据文件，token 刷新后会自动回写 |
| `pools.json` | ❌ | 池配置，可通过 Admin UI 创建管理 |
| `api_keys.json` | ❌ | API Key 配置，可通过 Admin UI 创建管理 |

## 示例文件

| 示例文件 | 说明 |
|---------|------|
| `config.example.json` | 主配置示例 |
| `credentials.example.social.json` | Social 认证凭据示例 |
| `credentials.example.idc.json` | IdC 认证凭据示例 |
| `credentials.example.multiple.json` | 多凭据格式示例 |
| `pools.example.json` | 池配置示例 |
| `api_keys.example.json` | API Key 配置示例 |

## 快速开始

1. 复制示例文件：
   ```bash
   cp config.example.json config.json
   cp credentials.example.social.json credentials.json  # 或使用 idc/multiple 版本
   ```

2. 编辑 `config.json`，修改以下字段：
   - `host`: Docker 部署时改为 `"0.0.0.0"`
   - `apiKey`: 你的 API Key（用于客户端认证）
   - `adminApiKey`: Admin API 密钥（用于访问管理后台）

3. 编辑 `credentials.json`，填入你的凭据：
   - `refreshToken`: 从 Kiro IDE 获取的刷新令牌
   - `expiresAt`: 过期时间（如果不确定，填一个过去的时间让程序自动刷新）
   - `authMethod`: 认证方式（`social` 或 `idc`）

4. 启动服务：
   ```bash
   cd ..  # 回到项目根目录
   docker-compose up -d
   ```

5. 访问 Admin UI：
   ```
   http://localhost:8990/admin
   ```

## 注意事项

- Docker 部署时 `host` 必须设为 `0.0.0.0` 以便容器外部访问
- 程序会自动回写 `credentials.json` 中的 token
- `pools.json` 和 `api_keys.json` 可以通过 Admin UI 动态创建，无需手动编辑
- 所有 `*.example.*` 文件仅供参考，不会被程序读取
