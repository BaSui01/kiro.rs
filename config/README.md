# 配置文件目录

此目录用于存放 kiro-rs 的配置文件，Docker 部署时会挂载到容器内。

## 文件说明

| 文件 | 必需 | 说明 |
|------|:----:|------|
| `config.json` | ✅ | 主配置文件 |
| `credentials.json` | ✅ | 凭据文件，token 刷新后会自动回写 |
| `pools.json` | ❌ | 池配置，可通过 Admin UI 创建管理 |
| `api_keys.json` | ❌ | API Key 配置，可通过 Admin UI 创建管理 |

## 示例文件

| 示例文件 | 说明 |
|---------|------|
| `config.example.json` | 主配置示例 |
| `credentials.example.json` | 凭据示例（支持 Social 和 IdC 认证） |

> **注意**：`pools.json` 和 `api_keys.json` 无需手动创建，可通过 Admin UI 动态管理。

## 快速开始

1. 复制示例文件：
   ```bash
   cp config.example.json config.json
   cp credentials.example.json credentials.json
   ```

2. 编辑 `config.json`，修改以下字段：
   - `host`: Docker 部署时改为 `"0.0.0.0"`
   - `adminApiKey`: Admin API 密钥（用于访问管理后台）

3. 编辑 `credentials.json`，填入你的凭据：
   - `refreshToken`: 从 Kiro IDE 获取的刷新令牌
   - `authMethod`: 认证方式（`social` 或 `idc`）
   - IdC 认证还需要 `clientId`、`clientSecret`、`region`

4. 启动服务：
   ```bash
   cd ..  # 回到项目根目录
   docker-compose up -d
   ```

5. 访问 Admin UI：
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
| `proxyUrl` | string | `null` | 全局代理地址 |
| `proxyUsername` | string | `null` | 代理认证用户名 |
| `proxyPassword` | string | `null` | 代理认证密码 |

## credentials.json 凭据格式

凭据文件必须使用 **JSON 数组格式**，即使只有一个凭据：

```json
[
  {
    "refreshToken": "你的刷新令牌",
    "authMethod": "social",
    "priority": 0
  }
]
```

### 凭据字段说明

| 字段 | 必需 | 说明 |
|------|:----:|------|
| `refreshToken` | ✅ | 刷新令牌（从 Kiro IDE 获取） |
| `authMethod` | ❌ | 认证方式：`social`（默认）或 `idc` |
| `priority` | ❌ | 优先级，数字越小优先级越高（默认 0） |
| `poolId` | ❌ | 所属池 ID（默认为 default 池） |
| `region` | ❌ | 凭据级区域配置（IdC 认证需要） |
| `clientId` | ❌ | OIDC Client ID（IdC 认证需要） |
| `clientSecret` | ❌ | OIDC Client Secret（IdC 认证需要） |
| `machineId` | ❌ | 机器 ID（可选，不填会自动生成） |
| `proxyUrl` | ❌ | 凭据级代理 URL |
| `proxyUsername` | ❌ | 凭据级代理用户名 |
| `proxyPassword` | ❌ | 凭据级代理密码 |

## 注意事项

- Docker 部署时 `host` 必须设为 `0.0.0.0` 以便容器外部访问
- 程序会自动回写 `credentials.json`（Token 刷新、统计数据）
- `pools.json` 和 `api_keys.json` 可通过 Admin UI 动态创建，无需手动编辑
