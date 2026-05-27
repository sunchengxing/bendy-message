mod storage;
pub mod election;
mod registry;
mod admin;

pub use election::LeaderElectionDO;

use bmsg_core::{SendMessageRequest, BatchSendMessageRequest, ApiResponse, BmsgError, MessageStorage, ServiceRegistry, Election, match_services, build_delivery_payload};
use worker::*;

fn bmsg_err(e: BmsgError) -> Error {
    Error::RustError(e.to_string())
}

#[event(fetch, respond_with_errors)]
pub async fn main(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    let router = Router::new();

    router
        // 消息接口
        .post_async("/api/v1/message/send", |req, ctx| async move {
            handle_send_message(req, ctx).await
        })
        .post_async("/api/v1/message/batch", |req, ctx| async move {
            handle_batch_send(req, ctx).await
        })
        .get_async("/api/v1/message/:id", |_, ctx| async move {
            handle_get_message(ctx).await
        })
        .get_async("/api/v1/message/list", |_, ctx| async move {
            handle_list_messages(ctx).await
        })
        .delete_async("/api/v1/message/:id", |_, ctx| async move {
            handle_delete_message(ctx).await
        })
        // 服务注册接口
        .post_async("/api/v1/service/register", |req, ctx| async move {
            handle_register_service(req, ctx).await
        })
        .post_async("/api/v1/service/unregister", |req, ctx| async move {
            handle_unregister_service(req, ctx).await
        })
        .post_async("/api/v1/service/heartbeat", |req, ctx| async move {
            handle_service_heartbeat(req, ctx).await
        })
        .get_async("/api/v1/service/list", |_, ctx| async move {
            handle_list_services(ctx).await
        })
        .get_async("/api/v1/service/:id/status", |_, ctx| async move {
            handle_service_status(ctx).await
        })
        // 节点接口
        .get_async("/api/v1/node/list", |_, ctx| async move {
            handle_list_nodes(ctx).await
        })
        .get_async("/api/v1/node/status", |_, ctx| async move {
            handle_node_status(ctx).await
        })
        .post_async("/api/v1/node/heartbeat", |_, ctx| async move {
            handle_node_heartbeat(ctx).await
        })
        // 管理面板
        .get_async("/admin", |_, ctx| async move {
            admin::serve_admin(ctx).await
        })
        .run(req, env)
        .await
}

async fn handle_send_message(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let body: SendMessageRequest = req.json().await?;
    let msg = body.into_message();
    let id = msg.id.clone();
    let persist = msg.persist;

    if persist {
        let db = ctx.env.d1("bmsg-db")?;
        let st = storage::D1Storage::new(db);
        st.store(&msg).await.map_err(bmsg_err)?;
    }

    // 路由投递：查找匹配服务并 POST
    let db = ctx.env.d1("bmsg-db")?;
    let reg = registry::KvRegistry::new(db);
    let services = reg.list().await.map_err(bmsg_err)?;
    let matched = match_services(&msg.target, &services);
    let payload = build_delivery_payload(&msg);

    let mut delivered = Vec::new();
    let mut failed = Vec::new();
    for svc in &matched {
        let payload_val = wasm_bindgen::JsValue::from_str(&serde_json::to_string(&payload).unwrap_or_default());
        let deliver_req = Request::new_with_init(&svc.endpoint, &worker::RequestInit {
            method: worker::Method::Post,
            body: Some(payload_val),
            ..Default::default()
        })?;
        match Fetch::Request(deliver_req).send().await {
            Ok(_) => delivered.push(svc.id.clone()),
            Err(e) => failed.push(serde_json::json!({"service_id": svc.id, "error": e.to_string()})),
        }
    }

    let resp = ApiResponse::success(serde_json::json!({
        "id": id,
        "delivered": delivered.len(),
        "failed": failed.len(),
        "persist": persist,
    }));
    Response::from_json(&resp)
}

async fn handle_batch_send(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let body: BatchSendMessageRequest = req.json().await?;
    let db = ctx.env.d1("bmsg-db")?;
    let st = storage::D1Storage::new(db);
    let mut results = Vec::new();

    for item in body.messages {
        let msg = item.into_message();
        let id = msg.id.clone();
        if msg.persist {
            let _ = st.store(&msg).await;
        }
        results.push(serde_json::json!({ "id": id, "status": "routed" }));
    }

    Response::from_json(&ApiResponse::success(results))
}

async fn handle_get_message(ctx: RouteContext<()>) -> Result<Response> {
    let id = ctx.param("id").ok_or_else(|| Error::RustError("missing id".into()))?;
    let db = ctx.env.d1("bmsg-db")?;
    let st = storage::D1Storage::new(db);
    match st.get(id).await {
        Ok(Some(msg)) => Response::from_json(&ApiResponse::success(msg)),
        Ok(None) => Response::from_json(&ApiResponse::<()>::error(1004, "message not found".into())),
        Err(e) => Response::from_json(&ApiResponse::<()>::error(1000, e.to_string())),
    }
}

async fn handle_list_messages(ctx: RouteContext<()>) -> Result<Response> {
    let db = ctx.env.d1("bmsg-db")?;
    let st = storage::D1Storage::new(db);
    let filter = bmsg_core::MessageFilter::default();
    match st.list(&filter, 1, 50).await {
        Ok(msgs) => Response::from_json(&ApiResponse::success(msgs)),
        Err(e) => Response::from_json(&ApiResponse::<()>::error(1000, e.to_string())),
    }
}

async fn handle_delete_message(ctx: RouteContext<()>) -> Result<Response> {
    let id = ctx.param("id").ok_or_else(|| Error::RustError("missing id".into()))?;
    let db = ctx.env.d1("bmsg-db")?;
    let st = storage::D1Storage::new(db);
    st.delete(id).await.map_err(bmsg_err)?;
    Response::from_json(&ApiResponse::success(serde_json::json!({"deleted": true})))
}

async fn handle_register_service(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let body: bmsg_core::RegisterRequest = req.json().await?;
    let db = ctx.env.d1("bmsg-db")?;
    let reg = registry::KvRegistry::new(db);
    match reg.register(&body).await {
        Ok(svc) => Response::from_json(&ApiResponse::success(svc)),
        Err(e) => Response::from_json(&ApiResponse::<()>::error(e.code(), e.to_string())),
    }
}

async fn handle_unregister_service(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let body: serde_json::Value = req.json().await?;
    let id = body["id"].as_str().unwrap_or("");
    let db = ctx.env.d1("bmsg-db")?;
    let reg = registry::KvRegistry::new(db);
    reg.unregister(id).await.map_err(bmsg_err)?;
    Response::from_json(&ApiResponse::success(serde_json::json!({"unregistered": true})))
}

async fn handle_service_heartbeat(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let body: serde_json::Value = req.json().await?;
    let id = body["id"].as_str().unwrap_or("");
    let db = ctx.env.d1("bmsg-db")?;
    let reg = registry::KvRegistry::new(db);
    reg.heartbeat(id).await.map_err(bmsg_err)?;
    Response::from_json(&ApiResponse::success(serde_json::json!({"heartbeat": "ok"})))
}

async fn handle_list_services(ctx: RouteContext<()>) -> Result<Response> {
    let db = ctx.env.d1("bmsg-db")?;
    let reg = registry::KvRegistry::new(db);
    match reg.list().await {
        Ok(svcs) => Response::from_json(&ApiResponse::success(svcs)),
        Err(e) => Response::from_json(&ApiResponse::<()>::error(1000, e.to_string())),
    }
}

async fn handle_service_status(ctx: RouteContext<()>) -> Result<Response> {
    let id = ctx.param("id").ok_or_else(|| Error::RustError("missing id".into()))?;
    let db = ctx.env.d1("bmsg-db")?;
    let reg = registry::KvRegistry::new(db);
    match reg.get_status(id).await {
        Ok(status) => Response::from_json(&ApiResponse::success(serde_json::json!({"status": status.to_string()}))),
        Err(e) => Response::from_json(&ApiResponse::<()>::error(e.code(), e.to_string())),
    }
}

async fn handle_list_nodes(ctx: RouteContext<()>) -> Result<Response> {
    let namespace = ctx.env.durable_object("bmsg-election")?;
    let elec = election::DoElection::new(namespace);
    match elec.get_nodes().await {
        Ok(nodes) => Response::from_json(&ApiResponse::success(nodes)),
        Err(e) => Response::from_json(&ApiResponse::<()>::error(1000, e.to_string())),
    }
}

async fn handle_node_status(ctx: RouteContext<()>) -> Result<Response> {
    let namespace = ctx.env.durable_object("bmsg-election")?;
    let elec = election::DoElection::new(namespace);
    let is_master = elec.is_master().await;
    Response::from_json(&ApiResponse::success(serde_json::json!({"is_master": is_master})))
}

async fn handle_node_heartbeat(ctx: RouteContext<()>) -> Result<Response> {
    let namespace = ctx.env.durable_object("bmsg-election")?;
    let elec = election::DoElection::new(namespace);
    elec.heartbeat().await.map_err(bmsg_err)?;
    Response::from_json(&ApiResponse::success(serde_json::json!({"heartbeat": "ok"})))
}
