use axum::http::StatusCode;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct ChatApiUpstreamError {
    pub status: StatusCode,
    pub body: Option<Value>,
}

impl ChatApiUpstreamError {
    fn new(status: StatusCode, body: Option<Value>) -> Self {
        Self { status, body }
    }
}

fn chat_api_base_url() -> String {
    std::env::var("CHAT_API_URL").unwrap_or_else(|_| "http://chat.localhost:8080".to_string())
}

fn chat_api_connect_base_url() -> String {
    // Many dev setups route everything through a single local ingress on 127.0.0.1:8080,
    // and use the Host header to select the service (auth.localhost, image.localhost, chat.localhost).
    std::env::var("CHAT_API_CONNECT_URL").unwrap_or_else(|_| "http://127.0.0.1:8080".to_string())
}

fn chat_api_host_header() -> String {
    std::env::var("CHAT_API_HOST").unwrap_or_else(|_| "chat.localhost".to_string())
}

fn bearer_headers(token: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    let auth_value = HeaderValue::from_str(&format!("Bearer {}", token)).unwrap();
    headers.insert(AUTHORIZATION, auth_value);
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers
}

fn connect_failed(url: &str, err: impl ToString) -> ChatApiUpstreamError {
    ChatApiUpstreamError::new(
        StatusCode::BAD_GATEWAY,
        Some(serde_json::json!({
            "error": "connect_failed",
            "detail": err.to_string(),
            "url": url
        })),
    )
}

pub async fn ws_ticket(token: &str, conversation_id: &str) -> Result<Value, ChatApiUpstreamError> {
    let connect_base = chat_api_connect_base_url();
    let host_header = chat_api_host_header();
    let url = format!("{}/api/v1/ws-ticket", connect_base.trim_end_matches('/'));

    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .header("Host", host_header)
        .headers(bearer_headers(token))
        .json(&serde_json::json!({ "conversation_id": conversation_id }))
        .send()
        .await
        .map_err(|e| connect_failed(&url, e))?;

    let status = StatusCode::from_u16(resp.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
    let body: Value = resp.json().await.map_err(|e| connect_failed(&url, e))?;
    if !status.is_success() {
        return Err(ChatApiUpstreamError::new(status, Some(body)));
    }

    // Normalize ws_url for the frontend if the api doesn't return it.
    // Docs say ws_url is included, but older builds returned only ticket+expires_at.
    if body.get("ws_url").is_some() {
        return Ok(body);
    }

    let ticket = body
        .get("ticket")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    if ticket.is_empty() {
        return Ok(body);
    }

    // Browser must connect to the routed host (not 127.0.0.1), so use CHAT_API_URL by default.
    let base_url = chat_api_base_url();
    let ws_base = std::env::var("CHAT_API_WS_URL").ok().unwrap_or_else(|| {
        if base_url.starts_with("https://") {
            base_url.replacen("https://", "wss://", 1)
        } else {
            base_url.replacen("http://", "ws://", 1)
        }
    });

    let ws_url = format!(
        "{}/api/v1/ws/{}?ticket={}",
        ws_base.trim_end_matches('/'),
        conversation_id,
        ticket
    );
    let mut out = body;
    if let Some(obj) = out.as_object_mut() {
        obj.insert("ws_url".to_string(), Value::String(ws_url));
    }
    Ok(out)
}

pub async fn health() -> Result<Value, ChatApiUpstreamError> {
    let connect_base = chat_api_connect_base_url();
    let host_header = chat_api_host_header();
    let url = format!("{}/health", connect_base.trim_end_matches('/'));

    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .header("Host", host_header)
        .send()
        .await
        .map_err(|e| connect_failed(&url, e))?;

    let status = StatusCode::from_u16(resp.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
    let body: Value = resp
        .json()
        .await
        .unwrap_or_else(|_| serde_json::json!({ "status": status.as_u16() }));
    if !status.is_success() {
        return Err(ChatApiUpstreamError::new(status, Some(body)));
    }
    Ok(body)
}

pub async fn list_messages(
    token: &str,
    conversation_id: &str,
    limit: i64,
    before: Option<String>,
    after: Option<String>,
) -> Result<Value, ChatApiUpstreamError> {
    let connect_base = chat_api_connect_base_url();
    let host_header = chat_api_host_header();
    let mut url = format!(
        "{}/api/v1/conversations/{}/messages?limit={}",
        connect_base.trim_end_matches('/'),
        conversation_id,
        limit.clamp(1, 100)
    );
    if let Some(before) = before {
        url.push_str(&format!("&before={}", before));
    }
    if let Some(after) = after {
        url.push_str(&format!("&after={}", after));
    }

    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .header("Host", host_header)
        .headers(bearer_headers(token))
        .send()
        .await
        .map_err(|e| connect_failed(&url, e))?;

    let status = StatusCode::from_u16(resp.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
    let body: Value = resp.json().await.map_err(|e| connect_failed(&url, e))?;
    if !status.is_success() {
        return Err(ChatApiUpstreamError::new(status, Some(body)));
    }
    Ok(body)
}

pub async fn send_message(
    token: &str,
    conversation_id: &str,
    content: String,
) -> Result<Value, ChatApiUpstreamError> {
    let connect_base = chat_api_connect_base_url();
    let host_header = chat_api_host_header();
    let url = format!(
        "{}/api/v1/conversations/{}/messages",
        connect_base.trim_end_matches('/'),
        conversation_id
    );

    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .header("Host", host_header)
        .headers(bearer_headers(token))
        .json(&serde_json::json!({
            "content": content,
            "message_type": "text",
            "metadata": {}
        }))
        .send()
        .await
        .map_err(|e| connect_failed(&url, e))?;

    let status = StatusCode::from_u16(resp.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
    let body: Value = resp.json().await.map_err(|e| connect_failed(&url, e))?;
    if !status.is_success() {
        return Err(ChatApiUpstreamError::new(status, Some(body)));
    }
    Ok(body)
}

pub async fn list_conversations(
    token: &str,
    limit: i64,
    before: Option<String>,
) -> Result<Value, ChatApiUpstreamError> {
    let connect_base = chat_api_connect_base_url();
    let host_header = chat_api_host_header();

    let mut url = format!(
        "{}/api/v1/conversations?limit={}",
        connect_base.trim_end_matches('/'),
        limit.clamp(1, 50)
    );
    if let Some(before) = before {
        url.push_str(&format!("&before={}", before));
    }

    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .header("Host", host_header)
        .headers(bearer_headers(token))
        .send()
        .await
        .map_err(|e| connect_failed(&url, e))?;

    let status = StatusCode::from_u16(resp.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
    let body: Value = resp.json().await.map_err(|e| connect_failed(&url, e))?;
    if !status.is_success() {
        return Err(ChatApiUpstreamError::new(status, Some(body)));
    }
    Ok(body)
}
