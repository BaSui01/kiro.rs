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
   cp api_keys.example.json api_keys.json  # 可选，用于 API Key 认证
   ```

2. 编辑 `config.json`，修改以下字段：
   - `host`: Docker 部署时改为 `"0.0.0.0"`
   - `adminApiKey`: Admin API 密钥（用于访问管理后台）

3. 编辑 `credentials.json`，填入你的凭据：
   - `refreshToken`: 从 Kiro IDE 获取的刷新令牌
   - `expiresAt`: 过期时间（如果不确定，填一个过去的时间让程序自动刷新）
   - `authMethod`: 认证方式（`social` 或 `idc`）

4. （可选）编辑 `api_keys.json`，配置 API Key 认证：
   - 用于客户端调用 API 时的认证
   - 也可以通过 Admin UI 动态管理

5. 启动服务：
   ```bash
   cd ..  # 回到项目根目录
   docker-compose up -d
   ```

6. 访问 Admin UI：
   ```
   http://localhost:8990/admin
   ```

## config.json 配置项说明

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `host` | string | `"127.0.0.1"` | 监听地址，Docker 部署时设为 `"0.0.0.0"` |
| `port` | number | `8080` | 监听端口 |
| `region` | string | `"us-east-1"` | AWS 区域 |
| `tlsBackend` | string | `"rustls"` | TLS 后端：`"rustls"` 或 `"native-tls"` |
| `adminApiKey` | string | `null` | Admin API 密钥，设置后启用管理后台 |
| `sessionCacheMaxCapacity` | number | `10000` | 会话缓存最大容量 |
| `sessionCacheTtlSecs` | number | `3600` | 会话缓存 TTL（秒） |
| `proxyUrl` | string | `null` | HTTP 代理地址 |
| `proxyUsername` | string | `null` | 代理认证用户名 |
| `proxyPassword` | string | `null` | 代理认证密码 |
| `countTokensApiUrl` | string | `null` | 外部 count_tokens API 地址 |
| `countTokensApiKey` | string | `null` | count_tokens API 密钥 |
| `countTokensAuthType` | string | `"x-api-key"` | count_tokens API 认证类型 |

## 注意事项

- Docker 部署时 `host` 必须设为 `0.0.0.0` 以便容器外部访问
- 程序会自动回写 `credentials.json` 中的 token
- `pools.json` 和 `api_keys.json` 可以通过 Admin UI 动态创建，无需手动编辑
- 所有 `*.example.*` 文件仅供参考，不会被程序读取
