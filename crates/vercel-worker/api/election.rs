use bmsg_core::{NodeInfo, NodeRole, BmsgError};
use reqwest::Client;

pub struct UpstashElection {
    rest_url: String,
    rest_token: String,
    client: Client,
}

impl UpstashElection {
    pub fn new(rest_url: String, rest_token: String) -> Self {
        Self { rest_url, rest_token, client: Client::new() }
    }

    async fn redis_get(&self, key: &str) -> Result<Option<serde_json::Value>, BmsgError> {
        let url = format!("{}/get/{}", self.rest_url.trim_end_matches('/'), key);
        let resp = self.client.get(&url)
            .header("Authorization", format!("Bearer {}", self.rest_token))
            .send().await
            .map_err(|e| BmsgError::ElectionError(e.to_string()))?;
        let val: serde_json::Value = resp.json().await.map_err(|e| BmsgError::ElectionError(e.to_string()))?;
        let result = val.get("result").cloned();
        Ok(result.filter(|v| !v.is_null()))
    }

    async fn redis_set(&self, key: &str, value: &str, ttl_secs: u64) -> Result<(), BmsgError> {
        let url = format!("{}/set/{}/{}?ttl={}", self.rest_url.trim_end_matches('/'), key, value, ttl_secs);
        self.client.post(&url)
            .header("Authorization", format!("Bearer {}", self.rest_token))
            .send().await
            .map_err(|e| BmsgError::ElectionError(e.to_string()))?;
        Ok(())
    }

    async fn redis_del(&self, key: &str) -> Result<(), BmsgError> {
        let url = format!("{}/del/{}", self.rest_url.trim_end_matches('/'), key);
        self.client.get(&url)
            .header("Authorization", format!("Bearer {}", self.rest_token))
            .send().await
            .map_err(|e| BmsgError::ElectionError(e.to_string()))?;
        Ok(())
    }

    pub async fn is_master(&self) -> bool {
        let node_id = get_node_id();
        match self.redis_get("bmsg:leader").await {
            Ok(Some(val)) => val.as_str().map(|s| s == &node_id).unwrap_or(false),
            _ => false,
        }
    }

    pub async fn campaign(&self) -> Result<bool, BmsgError> {
        let node_id = get_node_id();
        // Try SET with NX (only if not exists)
        let url = format!(
            "{}/set/bmsg:leader/{}?nx=true&ex=60",
            self.rest_url.trim_end_matches('/'), node_id
        );
        let resp = self.client.post(&url)
            .header("Authorization", format!("Bearer {}", self.rest_token))
            .send().await
            .map_err(|e| BmsgError::ElectionError(e.to_string()))?;
        let val: serde_json::Value = resp.json().await.map_err(|e| BmsgError::ElectionError(e.to_string()))?;
        Ok(val.get("result").is_some())
    }

    pub async fn resign(&self) -> Result<(), BmsgError> {
        self.redis_del("bmsg:leader").await
    }

    pub async fn get_master_info(&self) -> Result<Option<NodeInfo>, BmsgError> {
        match self.redis_get("bmsg:leader").await {
            Ok(Some(val)) => {
                let leader_id = val.as_str().unwrap_or("").to_string();
                match self.redis_get(&format!("bmsg:node:{}", leader_id)).await {
                    Ok(Some(info_val)) => {
                        let info: NodeInfo = serde_json::from_value(info_val)
                            .map_err(|e| BmsgError::ElectionError(e.to_string()))?;
                        Ok(Some(info))
                    }
                    _ => Ok(None),
                }
            }
            _ => Ok(None),
        }
    }

    pub async fn heartbeat(&self) -> Result<(), BmsgError> {
        let node_id = get_node_id();
        let now = chrono::Utc::now().timestamp();
        let is_master = self.is_master().await;
        let info = NodeInfo {
            id: node_id.clone(),
            role: if is_master { NodeRole::Master } else { NodeRole::Slave },
            platform: "vercel".into(),
            region: std::env::var("VERCEL_REGION").unwrap_or_else(|_| "auto".into()),
            started_at: now,
            last_heartbeat: now,
        };
        let info_json = serde_json::to_string(&info).map_err(|e| BmsgError::ElectionError(e.to_string()))?;
        self.redis_set(&format!("bmsg:node:{}", node_id), &info_json, 60).await?;
        if is_master {
            // Renew master lease
            self.redis_set("bmsg:leader", &node_id, 60).await?;
        }
        Ok(())
    }

    pub async fn get_nodes(&self) -> Result<Vec<NodeInfo>, BmsgError> {
        // Scan for node keys via KEYS command (suitable for small clusters)
        let url = format!("{}/keys/bmsg:node:*", self.rest_url.trim_end_matches('/'));
        let resp = self.client.get(&url)
            .header("Authorization", format!("Bearer {}", self.rest_token))
            .send().await
            .map_err(|e| BmsgError::ElectionError(e.to_string()))?;
        let val: serde_json::Value = resp.json().await.map_err(|e| BmsgError::ElectionError(e.to_string()))?;

        let mut nodes = Vec::new();
        if let Some(keys) = val.get("result").and_then(|r| r.as_array()) {
            for key in keys {
                if let Some(k) = key.as_str() {
                    if let Ok(Some(info_val)) = self.redis_get(k).await {
                        if let Ok(info) = serde_json::from_value::<NodeInfo>(info_val) {
                            nodes.push(info);
                        }
                    }
                }
            }
        }
        Ok(nodes)
    }
}

fn get_node_id() -> String {
    std::env::var("VERCEL_DEPLOYMENT_ID").unwrap_or_else(|_| uuid::Uuid::new_v4().to_string())
}
