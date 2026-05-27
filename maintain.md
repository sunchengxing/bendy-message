# maintain.md

## v0.2.1 - 2026-05-27

### 变更内容
- 侧边栏底部用户头像展示（GitHub头像URL + 首字母fallback）
- 顶部布局模式下的 topbar 用户头像展示
- 系统设置拆分为 Tab Bar 布局：管理员管理 + 系统配置
- 管理员表格增加「操作」列，dynamic管理员可删除，builtin灰显
- 系统配置 Tab：菜单位置切换（左侧 / 顶部），实时生效
- 修复密钥创建功能：增加返回值校验、显示生成密钥（可复制）、错误toast
- 发送消息平台改为下拉框，预置 Web/PC-Windows/PC-macOS/PC-Linux/Android/iOS
- 用户ID标记为非必填，不填时发送给应用下所有用户
- 侧边栏菜单添加 emoji 图标：💬🔗🖥️🔑⚙️
- 全局 toast 通知替代静默失败

### 影响范围
- admin/index.html — 管理面板前端全部改造

### 功能列表
- [x] 侧边栏用户头像
- [x] 系统设置 Tab Bar
- [x] 管理员操作列 + 删除
- [x] 系统配置（菜单位置切换）
- [x] 密钥创建 Bug 修复
- [x] 平台下啦框
- [x] 用户ID非必填
- [x] 菜单 emoji 图标

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
