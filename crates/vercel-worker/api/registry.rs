use bmsg_core::{RegisteredService, RegisterRequest, ServiceStatus, BmsgError};
use sqlx::PgPool;
use reqwest::Client;
use sha2::{Sha256, Digest};

pub struct UpstashRegistry {
    pool: PgPool,
    rest_url: String,
    rest_token: String,
    client: Client,
}

impl UpstashRegistry {
    pub fn new(pool: PgPool, rest_url: String, rest_token: String) -> Self {
        Self { pool, rest_url, rest_token, client: Client::new() }
    }

    async fn redis_set(&self, key: &str, value: &str, ttl_secs: u64) -> Result<(), BmsgError> {
        let url = format!("{}/set/{}/{}?ttl={}", self.rest_url.trim_end_matches('/'), key, value, ttl_secs);
        self.client.post(&url)
            .header("Authorization", format!("Bearer {}", self.rest_token))
            .send().await
            .map_err(|e| BmsgError::RegistryError(e.to_string()))?;
        Ok(())
    }

    async fn redis_del(&self, key: &str) -> Result<(), BmsgError> {
        let url = format!("{}/del/{}", self.rest_url.trim_end_matches('/'), key);
        self.client.get(&url)
            .header("Authorization", format!("Bearer {}", self.rest_token))
            .send().await
            .map_err(|e| BmsgError::RegistryError(e.to_string()))?;
        Ok(())
    }

    pub async fn register(&self, req: &RegisterRequest) -> Result<RegisteredService, BmsgError> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().timestamp();
        let secret_hash = hash_secret(&req.secret);
        let platforms_json = serde_json::to_string(&req.platforms).map_err(|e| BmsgError::RegistryError(e.to_string()))?;

        sqlx::query(
            "INSERT INTO bmsg_services (id, name, endpoint, app_package, platforms, secret_hash, status, last_heartbeat, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)"
        )
        .bind(&id)
        .bind(&req.name)
        .bind(&req.endpoint)
        .bind(&req.app_package)
        .bind(&platforms_json)
        .bind(&secret_hash)
        .bind("online")
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| BmsgError::RegistryError(e.to_string()))?;

        let svc = RegisteredService {
            id: id.clone(),
            name: req.name.clone(),
            endpoint: req.endpoint.clone(),
            app_package: req.app_package.clone(),
            platforms: req.platforms.clone(),
            secret_hash,
            status: ServiceStatus::Online,
            last_heartbeat: now,
            created_at: now,
        };

        let svc_json = serde_json::to_string(&svc).map_err(|e| BmsgError::RegistryError(e.to_string()))?;
        self.redis_set(&format!("bmsg:service:{}", id), &svc_json, 300).await?;

        Ok(svc)
    }

    pub async fn unregister(&self, id: &str) -> Result<(), BmsgError> {
        sqlx::query("DELETE FROM bmsg_services WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| BmsgError::RegistryError(e.to_string()))?;
        self.redis_del(&format!("bmsg:service:{}", id)).await?;
        Ok(())
    }

    pub async fn heartbeat(&self, id: &str) -> Result<(), BmsgError> {
        let now = chrono::Utc::now().timestamp();
        sqlx::query("UPDATE bmsg_services SET last_heartbeat = $1, status = 'online' WHERE id = $2")
            .bind(now)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| BmsgError::RegistryError(e.to_string()))?;
        Ok(())
    }

    pub async fn list(&self) -> Result<Vec<RegisteredService>, BmsgError> {
        let rows = sqlx::query_as::<_, ServiceRow>("SELECT * FROM bmsg_services ORDER BY created_at DESC")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| BmsgError::RegistryError(e.to_string()))?;

        rows.into_iter().map(|r| r.into_service()).collect()
    }

    pub async fn get_status(&self, id: &str) -> Result<ServiceStatus, BmsgError> {
        let row = sqlx::query_as::<_, (String,)>("SELECT status FROM bmsg_services WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| BmsgError::RegistryError(e.to_string()))?;

        match row {
            Some((status,)) => Ok(if status == "online" { ServiceStatus::Online } else { ServiceStatus::Offline }),
            None => Err(BmsgError::ServiceNotFound),
        }
    }
}

#[derive(sqlx::FromRow)]
struct ServiceRow {
    id: String,
    name: String,
    endpoint: String,
    app_package: String,
    platforms: String,
    secret_hash: String,
    status: String,
    last_heartbeat: i64,
    created_at: i64,
}

impl ServiceRow {
    fn into_service(self) -> Result<RegisteredService, BmsgError> {
        let platforms: Vec<String> = serde_json::from_str(&self.platforms)
            .map_err(|e| BmsgError::RegistryError(e.to_string()))?;
        let status = if self.status == "online" { ServiceStatus::Online } else { ServiceStatus::Offline };
        Ok(RegisteredService {
            id: self.id,
            name: self.name,
            endpoint: self.endpoint,
            app_package: self.app_package,
            platforms,
            secret_hash: self.secret_hash,
            status,
            last_heartbeat: self.last_heartbeat,
            created_at: self.created_at,
        })
    }
}

fn hash_secret(secret: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(secret.as_bytes());
    hex::encode(hasher.finalize())
}
