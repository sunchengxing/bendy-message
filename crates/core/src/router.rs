use crate::error::BmsgError;
use crate::message::{Message, Target};
use crate::registry::ServiceRegistry;
use crate::storage::MessageStorage;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// 路由结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteResult {
    pub message_id: String,
    pub delivered: Vec<String>,
    pub failed: Vec<RouteFailure>,
    pub stored: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteFailure {
    pub service_id: String,
    pub endpoint: String,
    pub error: String,
}

/// 路由引擎
pub struct Router<S: MessageStorage, R: ServiceRegistry> {
    storage: S,
    registry: R,
}

impl<S: MessageStorage, R: ServiceRegistry> Router<S, R> {
    pub fn new(storage: S, registry: R) -> Self {
        Self { storage, registry }
    }

    /// 路由消息：存储（如需）+ 投递到匹配服务
    pub async fn route(&self, msg: Message) -> Result<RouteResult, BmsgError> {
        let message_id = msg.id.clone();
        let persist = msg.persist;
        let mut stored = false;

        // 存储
        if persist {
            self.storage.store(&msg).await?;
            stored = true;
        }

        // 查找匹配的已注册服务
        let services = self.registry.list().await?;
        let mut delivered = Vec::new();
        let mut failed = Vec::new();

        let route_target = Target {
            platform: msg.target.platform.clone(),
            app_package: msg.target.app_package.clone(),
            user_id: msg.target.user_id.clone(),
            msg_type: msg.msg_type.clone(),
        };

        for svc in services {
            // 匹配检查：服务的 platforms 包含消息目标平台
            let platform_match = svc.platforms.contains(&route_target.platform)
                || svc.platforms.contains(&"*".to_string());
            let app_match = svc.app_package == route_target.app_package
                || svc.app_package == "*";

            if platform_match && app_match {
                // 投递消息到服务 endpoint
                match self.deliver(&svc.endpoint, &msg).await {
                    Ok(_) => delivered.push(svc.id.clone()),
                    Err(e) => failed.push(RouteFailure {
                        service_id: svc.id.clone(),
                        endpoint: svc.endpoint.clone(),
                        error: e.to_string(),
                    }),
                }
            }
        }

        Ok(RouteResult { message_id, delivered, failed, stored })
    }

    /// 批量路由
    pub async fn route_batch(&self, messages: Vec<Message>) -> Vec<Result<RouteResult, BmsgError>> {
        let mut results = Vec::with_capacity(messages.len());
        for msg in messages {
            results.push(self.route(msg).await);
        }
        results
    }

    /// 投递消息到目标 endpoint
    async fn deliver(&self, _endpoint: &str, _msg: &Message) -> Result<(), BmsgError> {
        // 投递逻辑由平台适配层实现，这里提供通用 HTTP POST
        // 实际在 cf-worker 和 vercel-worker 中各自实现
        Err(BmsgError::DeliveryNotImplemented)
    }
}

/// 可投递的路由器 trait（平台适配层实现实际 HTTP 投递）
#[async_trait(?Send)]
pub trait Deliverable {
    async fn deliver(&self, endpoint: &str, msg: &Message) -> Result<(), BmsgError>;
}
