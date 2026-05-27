use crate::error::BmsgError;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// 服务状态
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceStatus {
    Online,
    Offline,
}

impl std::fmt::Display for ServiceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Online => f.write_str("online"),
            Self::Offline => f.write_str("offline"),
        }
    }
}

/// 已注册服务
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisteredService {
    pub id: String,
    pub name: String,
    pub endpoint: String,
    pub app_package: String,
    pub platforms: Vec<String>,
    pub secret_hash: String,
    pub status: ServiceStatus,
    pub last_heartbeat: i64,
    pub created_at: i64,
}

/// 注册请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub name: String,
    pub endpoint: String,
    pub app_package: String,
    pub platforms: Vec<String>,
    pub secret: String,
}

/// 服务注册 trait — 各平台实现
#[async_trait(?Send)]
pub trait ServiceRegistry {
    async fn register(&self, req: &RegisterRequest) -> Result<RegisteredService, BmsgError>;
    async fn unregister(&self, id: &str) -> Result<(), BmsgError>;
    async fn heartbeat(&self, id: &str) -> Result<(), BmsgError>;
    async fn list(&self) -> Result<Vec<RegisteredService>, BmsgError>;
    async fn get_status(&self, id: &str) -> Result<ServiceStatus, BmsgError>;
}
