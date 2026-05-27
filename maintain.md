# maintain.md

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
- 管理面板入口（待实现 UI）

### 影响范围
- 全新项目，无影响

### 功能列表
- [x] 项目骨架搭建
- [x] 消息模型定义
- [x] 核心 trait 定义
- [x] 错误码体系
- [x] 路由引擎框架
- [x] CF Workers 适配层
- [x] Vercel 适配层
- [x] 数据库迁移文件
- [ ] 管理面板 UI
- [ ] 单元测试
- [ ] 集成测试
