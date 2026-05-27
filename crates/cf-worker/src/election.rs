use std::cell::RefCell;

use bmsg_core::{Election, NodeInfo, NodeRole, BmsgError};
use async_trait::async_trait;
use worker::*;

struct DoState {
    master_id: Option<String>,
    master_heartbeat: i64,
    nodes: Vec<NodeInfo>,
}

#[durable_object]
pub struct LeaderElectionDO {
    _state: State,
    _env: Env,
    inner: RefCell<DoState>,
}

impl LeaderElectionDO {
    fn now() -> i64 {
        chrono::Utc::now().timestamp()
    }

    fn upsert_node(state: &mut DoState, id: &str, role: NodeRole, region: &str, now: i64) {
        if let Some(node) = state.nodes.iter_mut().find(|n| n.id == id) {
            node.role = role;
            node.last_heartbeat = now;
        } else {
            state.nodes.push(NodeInfo {
                id: id.into(), role, platform: "cf".into(), region: region.into(),
                started_at: now, last_heartbeat: now,
            });
        }
    }
}

impl DurableObject for LeaderElectionDO {
    fn new(state: State, env: Env) -> Self {
        Self { _state: state, _env: env, inner: RefCell::new(DoState { master_id: None, master_heartbeat: 0, nodes: Vec::new() }) }
    }

    async fn fetch(&self, mut req: Request) -> Result<Response> {
        let path = req.path();
        let method = req.method();
        let now = Self::now();

        // Check master heartbeat timeout (60s)
        {
            let s = self.inner.borrow();
            if s.master_heartbeat > 0 && now - s.master_heartbeat > 60 {
                drop(s);
                let mut s = self.inner.borrow_mut();
                s.master_id = None;
                s.master_heartbeat = 0;
            }
        }

        match (method, path.as_str()) {
            (Method::Post, "/campaign") => {
                let body: serde_json::Value = req.json().await?;
                let node_id = body["id"].as_str().unwrap_or("").to_string();
                let region = body["region"].as_str().unwrap_or("unknown").to_string();

                let mut s = self.inner.borrow_mut();
                let won = if s.master_id.is_none() {
                    s.master_id = Some(node_id.clone());
                    s.master_heartbeat = now;
                    true
                } else {
                    false
                };
                Self::upsert_node(&mut s, &node_id, if won { NodeRole::Master } else { NodeRole::Slave }, &region, now);
                let master = s.master_id.clone();
                drop(s);

                Response::from_json(&serde_json::json!({"won": won, "master": master}))
            }
            (Method::Post, "/heartbeat") => {
                let body: serde_json::Value = req.json().await?;
                let node_id = body["id"].as_str().unwrap_or("").to_string();

                let mut s = self.inner.borrow_mut();
                if s.master_id.as_deref() == Some(node_id.as_str()) {
                    s.master_heartbeat = now;
                }
                if let Some(node) = s.nodes.iter_mut().find(|n| n.id == node_id) {
                    node.last_heartbeat = now;
                }
                drop(s);

                Response::from_json(&serde_json::json!({"ok": true}))
            }
            (Method::Post, "/resign") => {
                let body: serde_json::Value = req.json().await?;
                let node_id = body["id"].as_str().unwrap_or("").to_string();

                let mut s = self.inner.borrow_mut();
                if s.master_id.as_deref() == Some(node_id.as_str()) {
                    s.master_id = None;
                    s.master_heartbeat = 0;
                    if let Some(node) = s.nodes.iter_mut().find(|n| n.id == node_id) {
                        node.role = NodeRole::Slave;
                        node.last_heartbeat = now;
                    }
                }
                drop(s);

                Response::from_json(&serde_json::json!({"resigned": true}))
            }
            (Method::Get, "/is_master") => {
                let id = req.url()?.query_pairs().find(|(k, _)| k == "id").map(|(_, v)| v.to_string());
                let s = self.inner.borrow();
                let is = id.as_deref() == s.master_id.as_deref();
                drop(s);

                Response::from_json(&serde_json::json!({"is_master": is}))
            }
            (Method::Get, "/master") => {
                let s = self.inner.borrow();
                let info = s.master_id.as_ref().and_then(|mid| {
                    s.nodes.iter().find(|n| n.id == *mid).cloned()
                });
                drop(s);

                Response::from_json(&serde_json::json!({"master": info}))
            }
            (Method::Get, "/nodes") => {
                let mut s = self.inner.borrow_mut();
                s.nodes.retain(|n| now - n.last_heartbeat <= 60);
                let nodes = s.nodes.clone();
                drop(s);

                Response::from_json(&serde_json::json!({"nodes": nodes}))
            }
            _ => Response::from_json(&serde_json::json!({"error": "unknown route"})),
        }
    }
}

/// Election adapter using Durable Objects
pub struct DoElection {
    namespace: ObjectNamespace,
}

impl DoElection {
    pub fn new(namespace: ObjectNamespace) -> Self {
        Self { namespace }
    }

    async fn do_fetch(&self, path: &str, method: &str, body: Option<serde_json::Value>) -> Result<serde_json::Value, BmsgError> {
        let id = self.namespace.id_from_name("leader-election")
            .map_err(|e| BmsgError::ElectionError(e.to_string()))?;
        let stub = id.get_stub()
            .map_err(|e| BmsgError::ElectionError(e.to_string()))?;

        let mut init = RequestInit::default();
        init.method = match method {
            "POST" => Method::Post,
            _ => Method::Get,
        };
        if let Some(b) = body {
            init.body = Some(wasm_bindgen::JsValue::from_str(&serde_json::to_string(&b).unwrap_or_default()));
        }

        let req = Request::new_with_init(&format!("https://do.internal{}", path), &init)
            .map_err(|e| BmsgError::ElectionError(e.to_string()))?;
        let mut resp = stub.fetch_with_request(req).await
            .map_err(|e| BmsgError::ElectionError(e.to_string()))?;
        resp.json().await.map_err(|e| BmsgError::ElectionError(e.to_string()))
    }

    fn node_id() -> String {
        uuid::Uuid::new_v4().to_string()
    }
}

#[async_trait(?Send)]
impl Election for DoElection {
    async fn is_master(&self) -> bool {
        self.do_fetch(&format!("/is_master?id={}", Self::node_id()), "GET", None)
            .await
            .ok()
            .and_then(|v| v["is_master"].as_bool())
            .unwrap_or(false)
    }

    async fn campaign(&self) -> Result<bool, BmsgError> {
        let body = serde_json::json!({"id": Self::node_id(), "region": "auto"});
        self.do_fetch("/campaign", "POST", Some(body))
            .await
            .map(|v| v["won"].as_bool().unwrap_or(false))
    }

    async fn resign(&self) -> Result<(), BmsgError> {
        let body = serde_json::json!({"id": Self::node_id()});
        self.do_fetch("/resign", "POST", Some(body)).await?;
        Ok(())
    }

    async fn get_master_info(&self) -> Result<Option<NodeInfo>, BmsgError> {
        let v = self.do_fetch("/master", "GET", None).await?;
        if v["master"].is_null() {
            Ok(None)
        } else {
            let info: NodeInfo = serde_json::from_value(v["master"].clone())
                .map_err(|e| BmsgError::ElectionError(e.to_string()))?;
            Ok(Some(info))
        }
    }

    async fn heartbeat(&self) -> Result<(), BmsgError> {
        let body = serde_json::json!({"id": Self::node_id()});
        self.do_fetch("/heartbeat", "POST", Some(body)).await?;
        Ok(())
    }

    async fn get_nodes(&self) -> Result<Vec<NodeInfo>, BmsgError> {
        let v = self.do_fetch("/nodes", "GET", None).await?;
        serde_json::from_value(v["nodes"].clone())
            .map_err(|e| BmsgError::ElectionError(e.to_string()))
    }
}
