use crate::error::BmsgError;
use crate::message::{Message, MessageFilter};
use async_trait::async_trait;

/// 消息存储 trait — 各平台实现
#[async_trait(?Send)]
pub trait MessageStorage {
    async fn store(&self, msg: &Message) -> Result<(), BmsgError>;
    async fn get(&self, id: &str) -> Result<Option<Message>, BmsgError>;
    async fn list(&self, filter: &MessageFilter, page: u32, limit: u32) -> Result<Vec<Message>, BmsgError>;
    async fn delete(&self, id: &str) -> Result<(), BmsgError>;
    async fn cleanup_expired(&self) -> Result<u64, BmsgError>;
}
