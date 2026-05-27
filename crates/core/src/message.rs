use serde::{Deserialize, Serialize};

/// 消息类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "snake_case")]
pub enum MessageType {
    Notification,
    Message,
    Shell,
}

impl MessageType {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Notification => "notification",
            Self::Message => "message",
            Self::Shell => "shell",
        }
    }
}

impl std::fmt::Display for MessageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// 消息路由目标
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct Target {
    /// 目标平台：ios/android/web/api/*...
    pub platform: String,
    /// 应用包名：com.example.app
    pub app_package: String,
    /// 用户ID，"*" 表示广播
    pub user_id: String,
    /// 消息类型过滤
    pub msg_type: MessageType,
}

impl Target {
    /// 通配符匹配：字段为 "*" 时匹配任何值
    pub fn matches(&self, other: &Target) -> bool {
        let field_match = |pattern: &str, value: &str| {
            pattern == "*" || pattern == value
        };
        field_match(&self.platform, &other.platform)
            && field_match(&self.app_package, &other.app_package)
            && field_match(&self.user_id, &other.user_id)
            && self.msg_type == other.msg_type
    }
}

/// 消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// UUID
    pub id: String,
    /// 路由目标
    pub target: Target,
    /// JSON 内容
    pub content: serde_json::Value,
    /// 消息类型
    pub msg_type: MessageType,
    /// 是否存储
    pub persist: bool,
    /// 存储TTL（秒），None=永久
    pub ttl: Option<u64>,
    /// 创建时间戳（秒）
    pub created_at: i64,
    /// 过期时间戳（created_at + ttl）
    pub expires_at: Option<i64>,
}

impl Message {
    pub fn new(target: Target, content: serde_json::Value, persist: bool, ttl: Option<u64>) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            msg_type: target.msg_type.clone(),
            expires_at: ttl.map(|t| now + t as i64),
            target,
            content,
            persist,
            ttl,
            created_at: now,
        }
    }
}

/// 消息查询过滤
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MessageFilter {
    pub platform: Option<String>,
    pub app_package: Option<String>,
    pub user_id: Option<String>,
    pub msg_type: Option<MessageType>,
    pub since: Option<i64>,
    pub until: Option<i64>,
}

/// 发送消息请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendMessageRequest {
    pub target: Target,
    pub content: serde_json::Value,
    pub persist: Option<bool>,
    pub ttl: Option<u64>,
}

impl SendMessageRequest {
    pub fn into_message(self) -> Message {
        Message::new(self.target, self.content, self.persist.unwrap_or(true), self.ttl)
    }
}

/// 批量发送请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchSendMessageRequest {
    pub messages: Vec<SendMessageRequest>,
}

/// 统一 API 响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T: Serialize> {
    pub code: u16,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self { code: 0, message: "ok".into(), data: Some(data) }
    }

    pub fn error(code: u16, message: String) -> ApiResponse<()> {
        ApiResponse { code, message, data: None }
    }
}
