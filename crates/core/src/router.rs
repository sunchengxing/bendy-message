use crate::error::BmsgError;
use crate::message::{Message, Target};
use crate::registry::{RegisteredService, ServiceRegistry};
use crate::storage::MessageStorage;
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

/// 匹配已注册服务与消息目标
pub fn match_services(msg_target: &Target, services: &[RegisteredService]) -> Vec<RegisteredService> {
    services
        .iter()
        .filter(|svc| {
            let platform_match =
                svc.platforms.contains(&msg_target.platform) || svc.platforms.contains(&"*".to_string());
            let app_match = svc.app_package == msg_target.app_package || svc.app_package == "*";
            platform_match && app_match
        })
        .cloned()
        .collect()
}

/// 构建投递 payload（发送给目标服务的 JSON body）
pub fn build_delivery_payload(msg: &Message) -> serde_json::Value {
    serde_json::json!({
        "id": msg.id,
        "target": {
            "platform": msg.target.platform,
            "app_package": msg.target.app_package,
            "user_id": msg.target.user_id,
            "msg_type": msg.target.msg_type.as_str(),
        },
        "msg_type": msg.msg_type.as_str(),
        "content": msg.content,
        "persist": msg.persist,
        "created_at": msg.created_at,
    })
}

/// 路由引擎（纯逻辑，投递由平台适配层负责）
pub struct Router<S: MessageStorage, R: ServiceRegistry> {
    storage: S,
    registry: R,
}

impl<S: MessageStorage, R: ServiceRegistry> Router<S, R> {
    pub fn new(storage: S, registry: R) -> Self {
        Self { storage, registry }
    }

    /// 路由消息：存储（如需）+ 返回匹配服务列表
    pub async fn route(&self, msg: Message) -> Result<RouteResult, BmsgError> {
        let message_id = msg.id.clone();
        let persist = msg.persist;
        let mut stored = false;

        if persist {
            self.storage.store(&msg).await?;
            stored = true;
        }

        let services = self.registry.list().await?;
        let matched = match_services(&msg.target, &services);

        // 投递结果由调用方（平台适配层）填充
        let delivered: Vec<String> = matched.iter().map(|s| s.id.clone()).collect();

        Ok(RouteResult {
            message_id,
            delivered,
            failed: Vec::new(),
            stored,
        })
    }

    /// 返回匹配服务列表（含 endpoint），供平台适配层做实际投递
    pub async fn find_routes(&self, msg: &Message) -> Result<Vec<RegisteredService>, BmsgError> {
        let services = self.registry.list().await?;
        Ok(match_services(&msg.target, &services))
    }

    /// 批量路由
    pub async fn route_batch(&self, messages: Vec<Message>) -> Vec<Result<RouteResult, BmsgError>> {
        let mut results = Vec::with_capacity(messages.len());
        for msg in messages {
            results.push(self.route(msg).await);
        }
        results
    }
}
