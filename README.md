# bendy-message

统一消息中心 — 只做两件事：**存储消息** + **路由消息**

## 概述

bendy-message 是一个通用统一消息平台，所有应用通过 HTTP API 统一入口发消息，系统根据目标地址进行精准路由投递。

- 消息目标 = `平台` + `应用包名` + `用户ID` + `消息类型` + `内容(JSON)`
- 消息类型：`notification`(通知) / `message`(消息) / `shell`(命令)
- 标注存储的消息自动存储 + TTL 定时清理，不存储的直接路由透传
- 分布式 1主多从选举机制
- 主节点带后台管理面板

## 技术栈

| 层 | 技术 |
|---|------|
| 核心 | Rust |
| CF 部署 | Cloudflare Workers (WASM) + D1 + KV + Durable Objects |
| Vercel 部署 | vercel_runtime (Native Rust) + PostgreSQL + Upstash |
| 管理面板 | Petite-Vue |

## 业务前缀

- 数据表：`bmsg_`
- Redis/KV keys：`bmsg:`
  - `bmsg:leader` — 主节点信息
  - `bmsg:node:{id}` — 节点心跳
  - `bmsg:route:{app}:{platform}` — 路由规则
  - `bmsg:service:{id}` — 服务缓存

## API

### 消息接口

| Method | Path | 说明 |
|--------|------|------|
| POST | `/api/v1/message/send` | 发送消息 |
| POST | `/api/v1/message/batch` | 批量发送 |
| GET | `/api/v1/message/:id` | 查询消息 |
| GET | `/api/v1/message/list` | 消息列表 |
| DELETE | `/api/v1/message/:id` | 删除消息 |

### 服务注册接口

| Method | Path | 说明 |
|--------|------|------|
| POST | `/api/v1/service/register` | 注册服务 |
| POST | `/api/v1/service/unregister` | 注销服务 |
| POST | `/api/v1/service/heartbeat` | 心跳上报 |
| GET | `/api/v1/service/list` | 服务列表 |
| GET | `/api/v1/service/:id/status` | 服务状态 |

### 节点接口

| Method | Path | 说明 |
|--------|------|------|
| GET | `/api/v1/node/list` | 节点列表 |
| GET | `/api/v1/node/status` | 当前节点状态 |
| POST | `/api/v1/node/heartbeat` | 节点心跳 |

### 管理面板

| Method | Path | 说明 |
|--------|------|------|
| GET | `/admin` | 管理面板（仅主节点） |

## 项目结构

```
crates/
├── core/           # 核心库：消息模型 + 路由引擎 + trait
├── cf-worker/      # Cloudflare Workers 适配
└── vercel-worker/  # Vercel serverless 适配
admin/              # 管理面板 SPA
migrations/         # D1 + PostgreSQL 迁移
```

## 快速开始

### Cloudflare Workers

```bash
cd crates/cf-worker
wrangler d1 create bendy-message-db
# 更新 wrangler.toml 中的 database_id
wrangler d1 execute bendy-message-db --file=../../migrations/d1/001_init.sql
wrangler dev
```

### Vercel

```bash
cd crates/vercel-worker
# 配置环境变量
vercel env add DATABASE_URL
vercel env add UPSTASH_REDIS_REST_URL
vercel env add UPSTASH_REDIS_REST_TOKEN
vercel deploy
```
