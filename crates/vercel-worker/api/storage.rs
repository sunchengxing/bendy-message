use bmsg_core::{Message, MessageFilter, BmsgError};
use sqlx::PgPool;

pub struct PgStorage {
    pool: PgPool,
}

impl PgStorage {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn store(&self, msg: &Message) -> Result<(), BmsgError> {
        let content = serde_json::to_string(&msg.content).map_err(|e| BmsgError::StorageError(e.to_string()))?;
        sqlx::query(
            "INSERT INTO bmsg_messages (id, platform, app_package, user_id, msg_type, content, persist, ttl, created_at, expires_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)"
        )
        .bind(&msg.id)
        .bind(&msg.target.platform)
        .bind(&msg.target.app_package)
        .bind(&msg.target.user_id)
        .bind(msg.msg_type.as_str())
        .bind(&content)
        .bind(msg.persist)
        .bind(msg.ttl.map(|t| t as i64))
        .bind(msg.created_at)
        .bind(msg.expires_at)
        .execute(&self.pool)
        .await
        .map_err(|e| BmsgError::StorageError(e.to_string()))?;
        Ok(())
    }

    pub async fn get(&self, id: &str) -> Result<Option<Message>, BmsgError> {
        let row = sqlx::query_as::<_, MessageRow>("SELECT * FROM bmsg_messages WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| BmsgError::StorageError(e.to_string()))?;

        match row {
            Some(r) => Ok(Some(r.into_message()?)),
            None => Ok(None),
        }
    }

    pub async fn list(&self, _filter: &MessageFilter, page: u32, limit: u32) -> Result<Vec<Message>, BmsgError> {
        let offset = page.saturating_sub(1) * limit;
        let rows = sqlx::query_as::<_, MessageRow>(
            "SELECT * FROM bmsg_messages ORDER BY created_at DESC LIMIT $1 OFFSET $2"
        )
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| BmsgError::StorageError(e.to_string()))?;

        rows.into_iter().map(|r| r.into_message()).collect()
    }

    pub async fn delete(&self, id: &str) -> Result<(), BmsgError> {
        sqlx::query("DELETE FROM bmsg_messages WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| BmsgError::StorageError(e.to_string()))?;
        Ok(())
    }

    pub async fn cleanup_expired(&self) -> Result<u64, BmsgError> {
        let now = chrono::Utc::now().timestamp();
        let result = sqlx::query("DELETE FROM bmsg_messages WHERE expires_at IS NOT NULL AND expires_at < $1")
            .bind(now)
            .execute(&self.pool)
            .await
            .map_err(|e| BmsgError::StorageError(e.to_string()))?;
        Ok(result.rows_affected())
    }
}

#[derive(sqlx::FromRow)]
struct MessageRow {
    id: String,
    platform: String,
    app_package: String,
    user_id: String,
    msg_type: String,
    content: String,
    persist: bool,
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
            persist: self.persist,
            ttl: self.ttl.map(|t| t as u64),
            created_at: self.created_at,
            expires_at: self.expires_at,
        })
    }
}
