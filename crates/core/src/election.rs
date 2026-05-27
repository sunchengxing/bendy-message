use crate::error::BmsgError;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// 节点角色
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeRole {
    Master,
    Slave,
}

/// 节点信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    pub id: String,
    pub role: NodeRole,
    pub platform: String,
    pub region: String,
    pub started_at: i64,
    pub last_heartbeat: i64,
}

/// 选举 trait — 各平台实现
#[async_trait(?Send)]
pub trait Election {
    /// 当前节点是否为主节点
    async fn is_master(&self) -> bool;
    /// 竞选主节点
    async fn campaign(&self) -> Result<bool, BmsgError>;
    /// 主动让位
    async fn resign(&self) -> Result<(), BmsgError>;
    /// 获取主节点信息
    async fn get_master_info(&self) -> Result<Option<NodeInfo>, BmsgError>;
    /// 节点心跳
    async fn heartbeat(&self) -> Result<(), BmsgError>;
    /// 获取所有节点列表
    async fn get_nodes(&self) -> Result<Vec<NodeInfo>, BmsgError>;
}
