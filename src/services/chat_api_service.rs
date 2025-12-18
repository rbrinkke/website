use axum::http::StatusCode;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::models::chat_api_models::*;

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

async fn request<T: DeserializeOwned>(
    method: reqwest::Method,
    path: &str,
    token: Option<&str>,
    body: Option<Value>,
) -> Result<T, ChatApiUpstreamError> {
    let connect_base = chat_api_connect_base_url();
    let host_header = chat_api_host_header();
    let url = format!("{}{}", connect_base.trim_end_matches('/'), path);

    let client = reqwest::Client::new();
    let mut rb = client.request(method, &url).header("Host", host_header);

    if let Some(t) = token {
        rb = rb.headers(bearer_headers(t));
    }

    if let Some(b) = body {
        rb = rb.json(&b);
    }

    let resp = rb.send().await.map_err(|e| connect_failed(&url, e))?;
    let status = resp.status();
    let body_val: Value = resp.json().await.map_err(|e| connect_failed(&url, e))?;

    if !status.is_success() {
        return Err(ChatApiUpstreamError::new(status, Some(body_val)));
    }

    serde_json::from_value(body_val).map_err(|e| {
        ChatApiUpstreamError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            Some(serde_json::json!({
                "error": "deserialization_failed",
                "detail": e.to_string()
            })),
        )
    })
}

// --- Conversations ---

pub async fn list_conversations(
    token: &str,
    limit: i64,
    before: Option<String>,
) -> Result<ConversationList, ChatApiUpstreamError> {
    let mut path = format!("/api/v1/conversations?limit={}", limit.clamp(1, 100));
    if let Some(b) = before {
        path.push_str(&format!("&before={}", b));
    }
    request(reqwest::Method::GET, &path, Some(token), None).await
}

pub async fn create_conversation(
    token: &str,
    other_user_id: &str,
) -> Result<ConversationCreateResult, ChatApiUpstreamError> {
    request(
        reqwest::Method::POST,
        "/api/v1/conversations",
        Some(token),
        Some(serde_json::json!({ "other_user_id": other_user_id })),
    )
    .await
}

pub async fn get_unread_count(
    token: &str,
    conversation_id: &str,
) -> Result<UnreadCount, ChatApiUpstreamError> {
    let path = format!("/api/v1/conversations/{}/unread", conversation_id);
    request(reqwest::Method::GET, &path, Some(token), None).await
}

// --- Messages ---

pub async fn list_messages(
    token: &str,
    conversation_id: &str,
    limit: i64,
    before: Option<String>,
    after: Option<String>,
) -> Result<MessageList, ChatApiUpstreamError> {
    let mut path = format!(
        "/api/v1/conversations/{}/messages?limit={}",
        conversation_id,
        limit.clamp(1, 100)
    );
    if let Some(b) = before {
        path.push_str(&format!("&before={}", b));
    }
    if let Some(a) = after {
        path.push_str(&format!("&after={}", a));
    }
    request(reqwest::Method::GET, &path, Some(token), None).await
}

pub async fn send_message(
    token: &str,
    conversation_id: &str,
    content: String,
) -> Result<Message, ChatApiUpstreamError> {
    let path = format!("/api/v1/conversations/{}/messages", conversation_id);
    request(
        reqwest::Method::POST,
        &path,
        Some(token),
        Some(serde_json::json!({
            "content": content,
            "message_type": "text"
        })),
    )
    .await
}

pub async fn get_message(
    token: &str,
    conversation_id: &str,
    message_id: &str,
) -> Result<Message, ChatApiUpstreamError> {
    let path = format!(
        "/api/v1/conversations/{}/messages/{}",
        conversation_id, message_id
    );
    request(reqwest::Method::GET, &path, Some(token), None).await
}

pub async fn edit_message(
    token: &str,
    conversation_id: &str,
    message_id: &str,
    content: String,
) -> Result<Message, ChatApiUpstreamError> {
    let path = format!(
        "/api/v1/conversations/{}/messages/{}",
        conversation_id, message_id
    );
    request(
        reqwest::Method::PUT,
        &path,
        Some(token),
        Some(serde_json::json!({ "content": content })),
    )
    .await
}

pub async fn delete_message(
    token: &str,
    conversation_id: &str,
    message_id: &str,
) -> Result<StatusConfirmation, ChatApiUpstreamError> {
    let path = format!(
        "/api/v1/conversations/{}/messages/{}",
        conversation_id, message_id
    );
    request(reqwest::Method::DELETE, &path, Some(token), None).await
}

pub async fn search_messages(
    token: &str,
    conversation_id: &str,
    query: &str,
) -> Result<SearchResult, ChatApiUpstreamError> {
    let path = format!(
        "/api/v1/conversations/{}/search?query={}",
        conversation_id, query
    );
    request(reqwest::Method::GET, &path, Some(token), None).await
}

// --- Interactions ---

pub async fn add_reaction(
    token: &str,
    conversation_id: &str,
    message_id: &str,
    emoji: &str,
) -> Result<StatusConfirmation, ChatApiUpstreamError> {
    let path = format!(
        "/api/v1/conversations/{}/messages/{}/reactions",
        conversation_id, message_id
    );
    request(
        reqwest::Method::POST,
        &path,
        Some(token),
        Some(serde_json::json!({ "emoji": emoji })),
    )
    .await
}

pub async fn remove_reaction(
    token: &str,
    conversation_id: &str,
    message_id: &str,
    emoji: &str,
) -> Result<StatusConfirmation, ChatApiUpstreamError> {
    let path = format!(
        "/api/v1/conversations/{}/messages/{}/reactions/{}",
        conversation_id, message_id, emoji
    );
    request(reqwest::Method::DELETE, &path, Some(token), None).await
}

pub async fn get_reactions(
    token: &str,
    conversation_id: &str,
    message_id: &str,
) -> Result<ReactionCounts, ChatApiUpstreamError> {
    let path = format!(
        "/api/v1/conversations/{}/messages/{}/reactions",
        conversation_id, message_id
    );
    request(reqwest::Method::GET, &path, Some(token), None).await
}

pub async fn reply_to_message(
    token: &str,
    conversation_id: &str,
    message_id: &str,
    content: String,
) -> Result<Message, ChatApiUpstreamError> {
    let path = format!(
        "/api/v1/conversations/{}/messages/{}/reply",
        conversation_id, message_id
    );
    request(
        reqwest::Method::POST,
        &path,
        Some(token),
        Some(serde_json::json!({ "content": content })),
    )
    .await
}

pub async fn mark_as_seen(
    token: &str,
    conversation_id: &str,
    message_id: &str,
) -> Result<StatusConfirmation, ChatApiUpstreamError> {
    let path = format!(
        "/api/v1/conversations/{}/messages/{}/seen",
        conversation_id, message_id
    );
    request(reqwest::Method::POST, &path, Some(token), None).await
}

// --- Features ---

pub async fn pin_message(
    token: &str,
    conversation_id: &str,
    message_id: &str,
) -> Result<Message, ChatApiUpstreamError> {
    let path = format!(
        "/api/v1/conversations/{}/messages/{}/pin",
        conversation_id, message_id
    );
    request(reqwest::Method::POST, &path, Some(token), None).await
}

pub async fn unpin_message(
    token: &str,
    conversation_id: &str,
    message_id: &str,
) -> Result<StatusConfirmation, ChatApiUpstreamError> {
    let path = format!(
        "/api/v1/conversations/{}/messages/{}/pin",
        conversation_id, message_id
    );
    request(reqwest::Method::DELETE, &path, Some(token), None).await
}

pub async fn get_pinned_messages(
    token: &str,
    conversation_id: &str,
) -> Result<MessageList, ChatApiUpstreamError> {
    let path = format!("/api/v1/conversations/{}/pinned", conversation_id);
    request(reqwest::Method::GET, &path, Some(token), None).await
}

pub async fn create_poll(
    token: &str,
    conversation_id: &str,
    message_id: &str,
    question: &str,
    options: Vec<String>,
) -> Result<Poll, ChatApiUpstreamError> {
    let path = format!(
        "/api/v1/conversations/{}/messages/{}/polls",
        conversation_id, message_id
    );
    request(
        reqwest::Method::POST,
        &path,
        Some(token),
        Some(serde_json::json!({
            "question": question,
            "options": options
        })),
    )
    .await
}

pub async fn vote_on_poll(
    token: &str,
    conversation_id: &str,
    poll_id: &str,
    option_id: &str,
) -> Result<StatusConfirmation, ChatApiUpstreamError> {
    let path = format!(
        "/api/v1/conversations/{}/polls/{}/vote",
        conversation_id, poll_id
    );
    request(
        reqwest::Method::POST,
        &path,
        Some(token),
        Some(serde_json::json!({ "option_id": option_id })),
    )
    .await
}

pub async fn schedule_message(
    token: &str,
    conversation_id: &str,
    content: String,
    scheduled_for: String,
) -> Result<ScheduledMessage, ChatApiUpstreamError> {
    let path = format!("/api/v1/conversations/{}/scheduled", conversation_id);
    request(
        reqwest::Method::POST,
        &path,
        Some(token),
        Some(serde_json::json!({
            "content": content,
            "scheduled_for": scheduled_for
        })),
    )
    .await
}

pub async fn get_thread(
    token: &str,
    conversation_id: &str,
    thread_id: &str,
) -> Result<ThreadResponse, ChatApiUpstreamError> {
    let path = format!(
        "/api/v1/conversations/{}/threads/{}",
        conversation_id, thread_id
    );
    request(reqwest::Method::GET, &path, Some(token), None).await
}

// --- Realtime ---

pub async fn ws_ticket(
    token: &str,
    conversation_id: &str,
) -> Result<WsTicketResponse, ChatApiUpstreamError> {
    let path = "/api/v1/ws-ticket";
    let body = serde_json::json!({ "conversation_id": conversation_id });
    let mut res: WsTicketResponse =
        request(reqwest::Method::POST, path, Some(token), Some(body)).await?;

    if res.ws_url.is_none() {
        let base_url = chat_api_base_url();
        let ws_base = if base_url.starts_with("https://") {
            base_url.replacen("https://", "wss://", 1)
        } else {
            base_url.replacen("http://", "ws://", 1)
        };
        res.ws_url = Some(format!(
            "{}/api/v1/ws/{}?ticket={}",
            ws_base.trim_end_matches('/'),
            conversation_id,
            res.ticket
        ));
    }
    Ok(res)
}

pub async fn health() -> Result<Value, ChatApiUpstreamError> {
    request(reqwest::Method::GET, "/health", None, None).await
}