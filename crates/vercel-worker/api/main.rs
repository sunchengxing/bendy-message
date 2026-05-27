mod storage;
mod election;
mod registry;
mod admin;

use bmsg_core::{SendMessageRequest, BatchSendMessageRequest, ApiResponse, RegisterRequest};
use vercel_runtime::{service_fn, Error, Request, Response, ResponseBody};
use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let handler = |req: Request| async move {
        handle(req).await
    };
    vercel_runtime::run(service_fn(handler)).await
}

async fn handle(req: Request) -> Result<Response<ResponseBody>, Error> {
    let path = req.uri().path().to_string();
    let method = req.method().clone();

    // Extract body bytes before routing (hyper 1.x consumes body)
    let body_bytes = http_body_util::BodyExt::collect(req.into_body())
        .await
        .map_err(|e| Error::from(e.to_string()))?
        .to_bytes();
    let body_slice = body_bytes.as_ref();

    let database_url = std::env::var("DATABASE_URL").unwrap_or_default();
    let upstash_url = std::env::var("UPSTASH_REDIS_REST_URL").unwrap_or_default();
    let upstash_token = std::env::var("UPSTASH_REDIS_REST_TOKEN").unwrap_or_default();

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .map_err(|e| Error::from(e.to_string()))?;

    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

    match (method.as_str(), segments.as_slice()) {
        ("POST", ["api", "v1", "message", "send"]) => {
            let body: SendMessageRequest = serde_json::from_slice(body_slice).map_err(|e| Error::from(e.to_string()))?;
            let msg = body.into_message();
            let id = msg.id.clone();
            if msg.persist {
                storage::PgStorage::new(pool).store(&msg).await.map_err(|e| Error::from(e.to_string()))?;
            }
            json_response(ApiResponse::success(serde_json::json!({"id": id, "status": "routed"})))
        }
        ("POST", ["api", "v1", "message", "batch"]) => {
            let body: BatchSendMessageRequest = serde_json::from_slice(body_slice).map_err(|e| Error::from(e.to_string()))?;
            let st = storage::PgStorage::new(pool);
            let mut results = Vec::new();
            for item in body.messages {
                let msg = item.into_message();
                let id = msg.id.clone();
                if msg.persist { let _ = st.store(&msg).await; }
                results.push(serde_json::json!({"id": id, "status": "routed"}));
            }
            json_response(ApiResponse::success(results))
        }
        ("GET", ["api", "v1", "message", id]) => {
            let st = storage::PgStorage::new(pool);
            match st.get(id).await {
                Ok(Some(msg)) => json_response(ApiResponse::success(msg)),
                Ok(None) => json_response(ApiResponse::<()>::error(1004, "message not found".into())),
                Err(e) => json_response(ApiResponse::<()>::error(1000, e.to_string())),
            }
        }
        ("GET", ["api", "v1", "message", "list"]) => {
            let st = storage::PgStorage::new(pool);
            let filter = bmsg_core::MessageFilter::default();
            match st.list(&filter, 1, 50).await {
                Ok(msgs) => json_response(ApiResponse::success(msgs)),
                Err(e) => json_response(ApiResponse::<()>::error(1000, e.to_string())),
            }
        }
        ("DELETE", ["api", "v1", "message", id]) => {
            storage::PgStorage::new(pool).delete(id).await.map_err(|e| Error::from(e.to_string()))?;
            json_response(ApiResponse::success(serde_json::json!({"deleted": true})))
        }
        ("POST", ["api", "v1", "service", "register"]) => {
            let body: RegisterRequest = serde_json::from_slice(body_slice).map_err(|e| Error::from(e.to_string()))?;
            let reg = registry::UpstashRegistry::new(pool, upstash_url, upstash_token);
            match reg.register(&body).await {
                Ok(svc) => json_response(ApiResponse::success(svc)),
                Err(e) => json_response(ApiResponse::<()>::error(e.code(), e.to_string())),
            }
        }
        ("POST", ["api", "v1", "service", "unregister"]) => {
            let body: serde_json::Value = serde_json::from_slice(body_slice).map_err(|e| Error::from(e.to_string()))?;
            let id = body["id"].as_str().unwrap_or("");
            registry::UpstashRegistry::new(pool, upstash_url, upstash_token)
                .unregister(id).await.map_err(|e| Error::from(e.to_string()))?;
            json_response(ApiResponse::success(serde_json::json!({"unregistered": true})))
        }
        ("POST", ["api", "v1", "service", "heartbeat"]) => {
            let body: serde_json::Value = serde_json::from_slice(body_slice).map_err(|e| Error::from(e.to_string()))?;
            let id = body["id"].as_str().unwrap_or("");
            registry::UpstashRegistry::new(pool, upstash_url, upstash_token)
                .heartbeat(id).await.map_err(|e| Error::from(e.to_string()))?;
            json_response(ApiResponse::success(serde_json::json!({"heartbeat": "ok"})))
        }
        ("GET", ["api", "v1", "service", "list"]) => {
            let reg = registry::UpstashRegistry::new(pool, upstash_url, upstash_token);
            match reg.list().await {
                Ok(svcs) => json_response(ApiResponse::success(svcs)),
                Err(e) => json_response(ApiResponse::<()>::error(1000, e.to_string())),
            }
        }
        ("GET", ["api", "v1", "service", id, "status"]) => {
            let reg = registry::UpstashRegistry::new(pool, upstash_url, upstash_token);
            match reg.get_status(id).await {
                Ok(status) => json_response(ApiResponse::success(serde_json::json!({"status": status.to_string()}))),
                Err(e) => json_response(ApiResponse::<()>::error(e.code(), e.to_string())),
            }
        }
        ("GET", ["api", "v1", "node", "list"]) => {
            let elec = election::UpstashElection::new(upstash_url, upstash_token);
            match elec.get_nodes().await {
                Ok(nodes) => json_response(ApiResponse::success(nodes)),
                Err(e) => json_response(ApiResponse::<()>::error(1000, e.to_string())),
            }
        }
        ("GET", ["api", "v1", "node", "status"]) => {
            let elec = election::UpstashElection::new(upstash_url, upstash_token);
            let is_master = elec.is_master().await;
            json_response(ApiResponse::success(serde_json::json!({"is_master": is_master})))
        }
        ("POST", ["api", "v1", "node", "heartbeat"]) => {
            let elec = election::UpstashElection::new(upstash_url, upstash_token);
            elec.heartbeat().await.map_err(|e| Error::from(e.to_string()))?;
            json_response(ApiResponse::success(serde_json::json!({"heartbeat": "ok"})))
        }
        ("GET", ["admin"]) => admin::serve_admin(),
        _ => json_response(ApiResponse::<()>::error(1003, format!("route not found: {}", path))),
    }
}

fn json_response<T: serde::Serialize>(data: T) -> Result<Response<ResponseBody>, Error> {
    let json = serde_json::to_string(&data).map_err(|e| Error::from(e.to_string()))?;
    Response::builder()
        .header("Content-Type", "application/json")
        .body(ResponseBody::from(json))
        .map_err(|e| Error::from(e.to_string()))
}
