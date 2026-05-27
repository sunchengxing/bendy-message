use worker::*;
use serde::{Deserialize, Serialize};

const BUILTIN_ADMINS: &[&str] = &["fntp", "LengendXing", "sunchengxing", "chenyiqiu", "chenshiluan", "52bendy", "yokeay"];
const SESSION_TTL: u64 = 86400;
const HOST: &str = "bendy-message.dabendi66.workers.dev";

#[derive(Serialize, Deserialize, Clone)]
pub struct Session {
    pub username: String,
    pub created_at: i64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ApiKey {
    pub id: String,
    pub name: String,
    pub key: String,
    pub created_at: i64,
}

fn get_session_cookie(headers: &Headers) -> Option<String> {
    let cookie_header = headers.get("Cookie").ok().flatten()?;
    cookie_header.split(';').find_map(|c| {
        let parts: Vec<&str> = c.trim().splitn(2, '=').collect();
        if parts.len() == 2 && parts[0] == "bmsg_session" {
            Some(parts[1].to_string())
        } else {
            None
        }
    })
}

async fn get_blocked_admins(kv: &KvStore) -> Vec<String> {
    kv.get("bmsg:config:admins_blocked").text().await
        .ok().flatten()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

async fn get_all_admins(kv: &KvStore) -> Vec<String> {
    let blocked = get_blocked_admins(kv).await;
    let mut admins: Vec<String> = BUILTIN_ADMINS.iter()
        .filter(|u| !blocked.contains(&u.to_string()))
        .map(|s| s.to_string()).collect();
    if let Some(extra) = kv.get("bmsg:config:admins").text().await.ok().flatten() {
        if let Ok(dynamic) = serde_json::from_str::<Vec<String>>(&extra) {
            for u in dynamic {
                if !admins.contains(&u) && !blocked.contains(&u) {
                    admins.push(u);
                }
            }
        }
    }
    admins
}

async fn get_dynamic_admins(kv: &KvStore) -> Vec<String> {
    kv.get("bmsg:config:admins").text().await
        .ok().flatten()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub async fn check_auth(headers: &Headers, kv: &KvStore) -> Option<String> {
    let sid = get_session_cookie(headers)?;
    let session: Option<Session> = kv.get(&format!("bmsg:session:{}", sid)).json().await.ok().flatten()?;
    let admins = get_all_admins(kv).await;
    session.filter(|s| admins.contains(&s.username)).map(|s| s.username)
}

pub fn github_redirect(env: &Env) -> Result<Response> {
    let client_id = env.var("GITHUB_CLIENT_ID")?.to_string();
    let redirect_uri = format!("https://{}/api/auth/github/callback", HOST);
    let url_str = format!(
        "https://github.com/login/oauth/authorize?client_id={}&redirect_uri={}&scope=read:user",
        client_id, redirect_uri
    );
    let url: Url = url_str.parse()?;
    Response::redirect(url)
}

pub async fn github_callback(req: Request, env: &Env) -> Result<Response> {
    let url = req.url()?;
    let code = url.query_pairs()
        .find(|(k, _)| k == "code")
        .map(|(_, v)| v.to_string())
        .ok_or_else(|| Error::RustError("missing code".into()))?;

    let client_id = env.var("GITHUB_CLIENT_ID")?.to_string();
    let client_secret = env.var("GITHUB_CLIENT_SECRET")?.to_string();

    // Exchange code for token
    let token_body = serde_json::json!({
        "client_id": client_id,
        "client_secret": client_secret,
        "code": code,
    });
    let token_req = Request::new_with_init(
        "https://github.com/login/oauth/access_token",
        &RequestInit {
            method: Method::Post,
            body: Some(wasm_bindgen::JsValue::from_str(&serde_json::to_string(&token_body).unwrap_or_default())),
            ..Default::default()
        },
    )?;
    token_req.headers().set("Accept", "application/json")?;
    token_req.headers().set("Content-Type", "application/json")?;

    let mut token_resp = Fetch::Request(token_req).send().await?;
    let token_data: serde_json::Value = token_resp.json().await?;
    let access_token = token_data["access_token"]
        .as_str()
        .ok_or_else(|| Error::RustError("no access token from GitHub".into()))?;

    // Get user info
    let user_req = Request::new("https://api.github.com/user", Method::Get)?;
    user_req.headers().set("Authorization", &format!("Bearer {}", access_token))?;
    user_req.headers().set("User-Agent", "bendy-message")?;

    let mut user_resp = Fetch::Request(user_req).send().await?;
    let user_data: serde_json::Value = user_resp.json().await?;
    let username = user_data["login"]
        .as_str()
        .ok_or_else(|| Error::RustError("no login from GitHub".into()))?;

    // Check admin
    let kv = env.kv("bmsg-cache")?;
    let admins = get_all_admins(&kv).await;
    if !admins.contains(&username.to_string()) {
        return Ok(Response::builder()
            .with_status(403)
            .with_header("Content-Type", "text/html; charset=utf-8")?
            .empty());
    }

    // Create session
    let session_id = uuid::Uuid::new_v4().to_string();
    let session = Session {
        username: username.to_string(),
        created_at: chrono::Utc::now().timestamp(),
    };
    let session_json = serde_json::to_string(&session).map_err(|e| Error::RustError(e.to_string()))?;
    kv.put(&format!("bmsg:session:{}", session_id), &session_json)?
        .expiration_ttl(SESSION_TTL)
        .execute()
        .await?;

    Ok(Response::builder()
        .with_status(302)
        .with_header("Location", &format!("https://{}/admin", HOST))?
        .with_header("Set-Cookie", &format!("bmsg_session={}; Path=/; HttpOnly; Secure; SameSite=Lax; Max-Age={}", session_id, SESSION_TTL))?
        .empty())
}

pub async fn session_info(headers: &Headers, env: &Env) -> Result<Response> {
    let kv = env.kv("bmsg-cache")?;
    match check_auth(headers, &kv).await {
        Some(username) => Response::from_json(&serde_json::json!({"code": 0, "username": username})),
        None => Ok(Response::builder()
            .with_status(401)
            .with_header("Content-Type", "application/json")?
            .empty()),
    }
}

pub async fn logout(headers: &Headers, env: &Env) -> Result<Response> {
    if let Some(sid) = get_session_cookie(headers) {
        let kv = env.kv("bmsg-cache")?;
        let _ = kv.delete(&format!("bmsg:session:{}", sid)).await;
    }
    Ok(Response::builder()
        .with_status(302)
        .with_header("Location", &format!("https://{}/admin", HOST))?
        .with_header("Set-Cookie", "bmsg_session=; Path=/; HttpOnly; Secure; SameSite=Lax; Max-Age=0")?
        .empty())
}

// --- Admin management ---

pub async fn list_admins(env: &Env) -> Result<Response> {
    let kv = env.kv("bmsg-cache")?;
    let dynamic = get_dynamic_admins(&kv).await;
    let mut result: Vec<serde_json::Value> = BUILTIN_ADMINS.iter()
        .map(|u| serde_json::json!({"username": u, "source": "builtin"}))
        .collect();
    for u in dynamic {
        result.push(serde_json::json!({"username": u, "source": "dynamic"}));
    }
    Response::from_json(&serde_json::json!({"code": 0, "data": result}))
}

pub async fn add_admin(mut req: Request, env: &Env) -> Result<Response> {
    let body: serde_json::Value = req.json().await?;
    let username = body["username"].as_str().unwrap_or("").to_string();
    if username.is_empty() {
        return Response::from_json(&serde_json::json!({"code": 1003, "message": "username required"}));
    }
    let kv = env.kv("bmsg-cache")?;
    let mut dynamic = get_dynamic_admins(&kv).await;
    if !dynamic.contains(&username) && !BUILTIN_ADMINS.contains(&username.as_str()) {
        dynamic.push(username);
    }
    let json = serde_json::to_string(&dynamic).map_err(|e| Error::RustError(e.to_string()))?;
    kv.put("bmsg:config:admins", &json)?.execute().await?;
    Response::from_json(&serde_json::json!({"code": 0, "message": "ok"}))
}

pub async fn remove_admin(mut req: Request, env: &Env) -> Result<Response> {
    let body: serde_json::Value = req.json().await?;
    let username = body["username"].as_str().unwrap_or("").to_string();
    if username.is_empty() {
        return Response::from_json(&serde_json::json!({"code": 1003, "message": "username required"}));
    }
    let kv = env.kv("bmsg-cache")?;
    if BUILTIN_ADMINS.contains(&username.as_str()) {
        // Block builtin admin by adding to blocked list
        let mut blocked = get_blocked_admins(&kv).await;
        if !blocked.contains(&username) {
            blocked.push(username.clone());
        }
        let json = serde_json::to_string(&blocked).map_err(|e| Error::RustError(e.to_string()))?;
        kv.put("bmsg:config:admins_blocked", &json)?.execute().await?;
    } else {
        // Remove dynamic admin
        let mut dynamic = get_dynamic_admins(&kv).await;
        dynamic.retain(|u| u != &username);
        let json = serde_json::to_string(&dynamic).map_err(|e| Error::RustError(e.to_string()))?;
        kv.put("bmsg:config:admins", &json)?.execute().await?;
    }
    Response::from_json(&serde_json::json!({"code": 0, "message": "ok"}))
}

// --- API Key management ---

pub async fn list_keys(env: &Env) -> Result<Response> {
    let kv = env.kv("bmsg-cache")?;
    let keys: Vec<ApiKey> = kv.get("bmsg:config:apikeys").json().await.ok().flatten().unwrap_or_default();
    Response::from_json(&serde_json::json!({"code": 0, "data": keys}))
}

pub async fn create_key(mut req: Request, env: &Env) -> Result<Response> {
    let body: serde_json::Value = req.json().await?;
    let name = body["name"].as_str().unwrap_or("").to_string();
    if name.is_empty() {
        return Response::from_json(&serde_json::json!({"code": 1003, "message": "name required"}));
    }
    let kv = env.kv("bmsg-cache")?;
    let mut keys: Vec<ApiKey> = kv.get("bmsg:config:apikeys").json().await.ok().flatten().unwrap_or_default();
    let new_key = ApiKey {
        id: uuid::Uuid::new_v4().to_string(),
        name,
        key: format!("bmsg_{}", uuid::Uuid::new_v4().to_string().replace('-', "")),
        created_at: chrono::Utc::now().timestamp(),
    };
    let result = new_key.key.clone();
    keys.push(new_key);
    let json = serde_json::to_string(&keys).map_err(|e| Error::RustError(e.to_string()))?;
    kv.put("bmsg:config:apikeys", &json)?.execute().await?;
    Response::from_json(&serde_json::json!({"code": 0, "data": {"key": result}}))
}

pub async fn delete_key(mut req: Request, env: &Env) -> Result<Response> {
    let body: serde_json::Value = req.json().await?;
    let id = body["id"].as_str().unwrap_or("").to_string();
    let kv = env.kv("bmsg-cache")?;
    let mut keys: Vec<ApiKey> = kv.get("bmsg:config:apikeys").json().await.ok().flatten().unwrap_or_default();

    // Find the key value for cascading delete
    let key_val = keys.iter().find(|k| k.id == id).map(|k| k.key.clone());
    keys.retain(|k| k.id != id);
    let json = serde_json::to_string(&keys).map_err(|e| Error::RustError(e.to_string()))?;
    kv.put("bmsg:config:apikeys", &json)?.execute().await?;

    // Cascade: delete services registered with this key
    if let Some(kv_val) = key_val {
        let db = env.d1("bmsg-db")?;
        let hash = crate::registry::simple_hash(&kv_val);
        db.prepare("DELETE FROM bmsg_services WHERE secret_hash = ?").bind(&[wasm_bindgen::JsValue::from_str(&hash)])?.run().await.ok();
    }

    Response::from_json(&serde_json::json!({"code": 0, "message": "ok"}))
}

/// Validate that a secret matches a known API key
pub async fn validate_api_key(secret: &str, kv: &KvStore) -> bool {
    let keys: Vec<ApiKey> = kv.get("bmsg:config:apikeys").json().await.ok().flatten().unwrap_or_default();
    keys.iter().any(|k| k.key == secret)
}
