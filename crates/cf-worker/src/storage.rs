use bmsg_core::{Message, MessageFilter, MessageStorage, BmsgError};
use async_trait::async_trait;
use worker::*;
use wasm_bindgen::JsValue;

pub struct D1Storage {
    db: D1Database,
}

impl D1Storage {
    pub fn new(db: D1Database) -> Self {
        Self { db }
    }
}

#[async_trait(?Send)]
impl MessageStorage for D1Storage {
    async fn store(&self, msg: &Message) -> Result<(), BmsgError> {
        let content = serde_json::to_string(&msg.content).map_err(|e| BmsgError::StorageError(e.to_string()))?;
        let persist_i32: i32 = if msg.persist { 1 } else { 0 };
        self.db.prepare(
            "INSERT INTO bmsg_messages (id, platform, app_package, user_id, msg_type, content, persist, ttl, created_at, expires_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)"
        )
        .bind(&[
            JsValue::from_str(&msg.id),
            JsValue::from_str(&msg.target.platform),
            JsValue::from_str(&msg.target.app_package),
            JsValue::from_str(&msg.target.user_id),
            JsValue::from_str(msg.msg_type.as_str()),
            JsValue::from_str(&content),
            JsValue::from_f64(persist_i32 as f64),
            msg.ttl.map(|t| JsValue::from_f64(t as f64)).unwrap_or(JsValue::NULL),
            JsValue::from_f64(msg.created_at as f64),
            msg.expires_at.map(|t| JsValue::from_f64(t as f64)).unwrap_or(JsValue::NULL),
        ])
        .map_err(|e| BmsgError::StorageError(e.to_string()))?
        .run()
        .await
        .map_err(|e| BmsgError::StorageError(e.to_string()))?;
        Ok(())
    }

    async fn get(&self, id: &str) -> Result<Option<Message>, BmsgError> {
        let result = self.db.prepare("SELECT * FROM bmsg_messages WHERE id = ?1")
            .bind(&[JsValue::from_str(id)])
            .map_err(|e| BmsgError::StorageError(e.to_string()))?
            .first::<MessageRow>(None)
            .await
            .map_err(|e| BmsgError::StorageError(e.to_string()))?;

        match result {
            Some(row) => Ok(Some(row.into_message()?)),
            None => Ok(None),
        }
    }

    async fn list(&self, _filter: &MessageFilter, page: u32, limit: u32) -> Result<Vec<Message>, BmsgError> {
        let offset = (page.saturating_sub(1)) * limit;
        let result = self.db.prepare("SELECT * FROM bmsg_messages ORDER BY created_at DESC LIMIT ?1 OFFSET ?2")
            .bind(&[JsValue::from_f64(limit as f64), JsValue::from_f64(offset as f64)])
            .map_err(|e| BmsgError::StorageError(e.to_string()))?
            .all()
            .await
            .map_err(|e| BmsgError::StorageError(e.to_string()))?;

        let rows: Vec<MessageRow> = result.results().map_err(|e| BmsgError::StorageError(e.to_string()))?;
        rows.into_iter().map(|r| r.into_message()).collect()
    }

    async fn delete(&self, id: &str) -> Result<(), BmsgError> {
        self.db.prepare("DELETE FROM bmsg_messages WHERE id = ?1")
            .bind(&[JsValue::from_str(id)])
            .map_err(|e| BmsgError::StorageError(e.to_string()))?
            .run()
            .await
            .map_err(|e| BmsgError::StorageError(e.to_string()))?;
        Ok(())
    }

    async fn cleanup_expired(&self) -> Result<u64, BmsgError> {
        let now = chrono::Utc::now().timestamp();
        let result = self.db.prepare("DELETE FROM bmsg_messages WHERE expires_at IS NOT NULL AND expires_at < ?1")
            .bind(&[JsValue::from_f64(now as f64)])
            .map_err(|e| BmsgError::StorageError(e.to_string()))?
            .run()
            .await
            .map_err(|e| BmsgError::StorageError(e.to_string()))?;
        let meta = result.meta().map_err(|e| BmsgError::StorageError(e.to_string()))?;
        Ok(meta.and_then(|m| m.changes).unwrap_or(0) as u64)
    }
}

#[derive(serde::Deserialize)]
struct MessageRow {
    id: String,
    platform: String,
    app_package: String,
    user_id: String,
    msg_type: String,
    content: String,
    persist: i32,
    ttl: Option<i64>,
    created_at: i64,
    expires_at: Option<i64>,
}

impl MessageRow {
    fn into_message(self) -> Result<Message, BmsgError> {
        let msg_type = match self.msg_type.as_str() {
            "notification" => bmsg_core::MessageType::Notification,
            "message" => bmsg_core::MessageType::Message,
            "shell" => bmsg_core::MessageType::Shell,
            _ => return Err(BmsgError::StorageError(format!("invalid msg_type: {}", self.msg_type))),
        };
        let content: serde_json::Value = serde_json::from_str(&self.content)
            .map_err(|e| BmsgError::StorageError(e.to_string()))?;
        Ok(Message {
            id: self.id,
            target: bmsg_core::Target {
                platform: self.platform,
                app_package: self.app_package,
                user_id: self.user_id,
                msg_type: msg_type.clone(),
            },
            content,
            msg_type,
            persist: self.persist != 0,
            ttl: self.ttl.map(|t| t as u64),
            created_at: self.created_at,
            expires_at: self.expires_at,
        })
    }
}
