# maintain.md

## v0.2.0 - 2026-05-27

### 变更内容
- 路由投递：消息发送时查找匹配服务并 POST 到 endpoint
- CF 侧使用 Fetch::Request + RequestInit 实现 HTTP 投递
- Vercel 侧使用 reqwest::Client 实现 HTTP 投递
- CF Durable Object 选举完整实现（LeaderElectionDO）
- DO 内部管理 master 节点、心跳、节点列表，60s 超时自动重选
- DoElection adapter 通过 DO stub 通信
- 核心 crate 单元测试 28 项全部通过
- 移除废弃的 DeliveryNotImplemented 错误变体
- 全 workspace 0 error 编译通过

### 影响范围
- core/router.rs — 纯函数 match_services / build_delivery_payload
- core/error.rs — 移除 DeliveryNotImplemented
- cf-worker/election.rs — 从 stub 升级为完整 DO 实现
- cf-worker/lib.rs — handle_send_message 增加投递逻辑
- vercel-worker/main.rs — message/send 增加投递逻辑
- core/tests/ — 新增 28 项单元测试

### 功能列表
- [x] 项目骨架搭建
- [x] 消息模型定义
- [x] 核心 trait 定义
- [x] 错误码体系
- [x] 路由引擎 + 投递逻辑
- [x] CF Workers 适配层（含 DO 选举）
- [x] Vercel 适配层
- [x] 数据库迁移文件
- [x] 管理面板 UI
- [x] 单元测试（28 项通过）
- [ ] 集成测试

## v0.1.0 - 2026-05-27

### 变更内容
- 初始化项目骨架
- Cargo workspace 结构：core + cf-worker + vercel-worker
- 核心消息模型定义（Message, Target, MessageType）
- 核心 trait 定义（MessageStorage, Election, ServiceRegistry）
- 路由引擎框架
- 统一错误码定义
- CF Workers 适配层（D1 + KV + DO）
- Vercel 适配层（PostgreSQL + Upstash）
- D1 / PostgreSQL 迁移文件
- HTTP API 完整路由定义
- 管理面板入口

### 影响范围
- 全新项目，无影响
