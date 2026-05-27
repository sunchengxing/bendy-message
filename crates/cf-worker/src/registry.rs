use bmsg_core::{ServiceRegistry, RegisteredService, RegisterRequest, ServiceStatus, BmsgError};
use async_trait::async_trait;
use worker::*;
use wasm_bindgen::JsValue;

pub struct KvRegistry {
    db: D1Database,
}

impl KvRegistry {
    pub fn new(db: D1Database) -> Self {
        Self { db }
    }
}

#[async_trait(?Send)]
impl ServiceRegistry for KvRegistry {
    async fn register(&self, req: &RegisterRequest) -> Result<RegisteredService, BmsgError> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().timestamp();
        let secret_hash = simple_hash(&req.secret);

        let platforms_json = serde_json::to_string(&req.platforms).map_err(|e| BmsgError::RegistryError(e.to_string()))?;

        self.db.prepare(
            "INSERT INTO bmsg_services (id, name, endpoint, app_package, platforms, secret_hash, status, last_heartbeat, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)"
        )
        .bind(&[
            JsValue::from_str(&id),
            JsValue::from_str(&req.name),
            JsValue::from_str(&req.endpoint),
            JsValue::from_str(&req.app_package),
            JsValue::from_str(&platforms_json),
            JsValue::from_str(&secret_hash),
            JsValue::from_str("online"),
            JsValue::from_f64(now as f64),
            JsValue::from_f64(now as f64),
        ])
        .map_err(|e| BmsgError::RegistryError(e.to_string()))?
        .run()
        .await
        .map_err(|e| BmsgError::RegistryError(e.to_string()))?;

        Ok(RegisteredService {
            id,
            name: req.name.clone(),
            endpoint: req.endpoint.clone(),
            app_package: req.app_package.clone(),
            platforms: req.platforms.clone(),
            secret_hash,
            status: ServiceStatus::Online,
            last_heartbeat: now,
            created_at: now,
        })
    }

    async fn unregister(&self, id: &str) -> Result<(), BmsgError> {
        self.db.prepare("DELETE FROM bmsg_services WHERE id = ?1")
            .bind(&[JsValue::from_str(id)])
            .map_err(|e| BmsgError::RegistryError(e.to_string()))?
            .run()
            .await
            .map_err(|e| BmsgError::RegistryError(e.to_string()))?;
        Ok(())
    }

    async fn heartbeat(&self, id: &str) -> Result<(), BmsgError> {
        let now = chrono::Utc::now().timestamp();
        self.db.prepare("UPDATE bmsg_services SET last_heartbeat = ?1, status = 'online' WHERE id = ?2")
            .bind(&[JsValue::from_f64(now as f64), JsValue::from_str(id)])
            .map_err(|e| BmsgError::RegistryError(e.to_string()))?
            .run()
            .await
            .map_err(|e| BmsgError::RegistryError(e.to_string()))?;
        Ok(())
    }

    async fn list(&self) -> Result<Vec<RegisteredService>, BmsgError> {
        let result = self.db.prepare("SELECT * FROM bmsg_services")
            .bind(&[])
            .map_err(|e| BmsgError::RegistryError(e.to_string()))?
            .all()
            .await
            .map_err(|e| BmsgError::RegistryError(e.to_string()))?;

        let rows: Vec<ServiceRow> = result.results().map_err(|e| BmsgError::RegistryError(e.to_string()))?;
        rows.into_iter().map(|r| r.into_service()).collect()
    }

    async fn get_status(&self, id: &str) -> Result<ServiceStatus, BmsgError> {
        let result = self.db.prepare("SELECT status FROM bmsg_services WHERE id = ?1")
            .bind(&[JsValue::from_str(id)])
            .map_err(|e| BmsgError::RegistryError(e.to_string()))?
            .first::<StatusRow>(None)
            .await
            .map_err(|e| BmsgError::RegistryError(e.to_string()))?;

        match result {
            Some(row) => Ok(if row.status == "online" { ServiceStatus::Online } else { ServiceStatus::Offline }),
            None => Err(BmsgError::ServiceNotFound),
        }
    }
}

#[derive(serde::Deserialize)]
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

#[derive(serde::Deserialize)]
struct StatusRow {
    status: String,
}

pub fn simple_hash(input: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    input.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}
