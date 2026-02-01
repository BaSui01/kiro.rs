FROM node:22-alpine AS frontend-builder

WORKDIR /app/admin-ui
COPY admin-ui/package.json ./
RUN npm install -g pnpm && pnpm install
COPY admin-ui ./
RUN pnpm build

FROM rust:1.92-alpine AS builder

RUN apk add --no-cache musl-dev openssl-dev openssl-libs-static

WORKDIR /app
COPY Cargo.toml Cargo.lock* ./
COPY src ./src
COPY --from=frontend-builder /app/admin-ui/dist /app/admin-ui/dist

RUN cargo build --release

FROM alpine:3.21

RUN apk add --no-cache ca-certificates

WORKDIR /app
COPY --from=builder /app/target/release/kiro-rs /app/kiro-rs

VOLUME ["/app/config"]

EXPOSE 8990

# 配置文件说明（默认路径 config/）：
# - config/config.json: 主配置文件（必需）
# - config/credentials.json: 凭据文件（必需）
# - config/pools.json: 池配置文件（可选，用于凭据池管理）
# - config/api_keys.json: API Key 配置文件（可选，用于多 API Key 管理）

# 使用默认配置路径（config/config.json 和 config/credentials.json）
CMD ["./kiro-rs"]
