use axum::{
    extract::{Path, Query},
    http::{header, HeaderMap, StatusCode},
    Json,
};
use serde::Deserialize;
use serde_json::Value;

use crate::services::chat_api_service::{self, ChatApiUpstreamError};

/// P3.3: Robust cookie parsing
/// Handles both "; " and ";" separators and trims whitespace properly
fn extract_access_token(headers: &HeaderMap) -> Result<String, StatusCode> {
    let cookie_str = headers
        .get(header::COOKIE)
        .and_then(|hv| hv.to_str().ok())
        .unwrap_or("");

    // Split by ';' (handles both "; " and ";" separators) and trim whitespace
    cookie_str
        .split(';')
        .map(|part| part.trim())
        .find_map(|part| part.strip_prefix("access_token="))
        .map(|t| t.trim().to_string())
        .ok_or_else(|| {
            tracing::debug!("No access_token cookie found in request");
            StatusCode::UNAUTHORIZED
        })
}

fn map_chat_error(e: ChatApiUpstreamError) -> (StatusCode, Json<Value>) {
    tracing::warn!(status = %e.status, body = ?e.body, "chat_api_upstream_error");
    (
        e.status,
        Json(e.body.unwrap_or_else(|| serde_json::json!({ "error": "upstream_error" }))),
    )
}

#[derive(Debug, Deserialize)]
pub struct ListMessagesQuery {
    limit: Option<i64>,
    before: Option<String>,
    after: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SendMessageBody {
    content: String,
}

#[derive(Debug, Deserialize)]
pub struct WsTicketBody {
    conversation_id: String,
}

#[derive(Debug, Deserialize)]
pub struct ResolveChatQuery {
    pub local_conversation_id: String,
}

#[derive(Debug, Deserialize)]
pub struct ReactionBody {
    pub emoji: String,
}

#[derive(Debug, Deserialize)]
pub struct PollCreateBody {
    pub question: String,
    pub options: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct PollVoteBody {
    pub option_id: String,
}

#[derive(Debug, Deserialize)]
pub struct ScheduleMessageBody {
    pub content: String,
    pub scheduled_for: String,
}

pub async fn ws_ticket_handler(
    headers: HeaderMap,
    Json(body): Json<WsTicketBody>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let token = extract_access_token(&headers).map_err(|s| (s, Json(serde_json::json!({ "error": "unauthorized" }))))?;
    chat_api_service::ws_ticket(&token, &body.conversation_id)
        .await
        .map(|v| Json(serde_json::to_value(v).unwrap()))
        .map_err(map_chat_error)
}

pub async fn health_handler() -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    chat_api_service::health().await.map(Json).map_err(map_chat_error)
}

pub async fn list_conversations_handler(
    headers: HeaderMap,
    Query(q): Query<ListMessagesQuery>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let token = extract_access_token(&headers).map_err(|s| (s, Json(serde_json::json!({ "error": "unauthorized" }))))?;
    chat_api_service::list_conversations(&token, q.limit.unwrap_or(50), q.before)
        .await
        .map(|v| Json(serde_json::to_value(v).unwrap()))
        .map_err(map_chat_error)
}

pub async fn resolve_conversation_handler(
    headers: HeaderMap,
    Query(q): Query<ResolveChatQuery>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let token = extract_access_token(&headers).map_err(|s| (s, Json(serde_json::json!({ "error": "unauthorized" }))))?;

    let conversations = chat_api_service::list_conversations(&token, 100, None)
        .await
        .map_err(map_chat_error)?;

    let mut resolved_id: Option<String> = None;
    let mut match_kind: Option<String> = None;

    for c in &conversations.conversations {
        if c.id == q.local_conversation_id {
            resolved_id = Some(c.id.clone());
            match_kind = Some("id".to_string());
            break;
        }
        if c.external_id.as_deref() == Some(&q.local_conversation_id) {
            resolved_id = Some(c.id.clone());
            match_kind = Some("external_id".to_string());
            break;
        }
    }

    if let Some(chat_conversation_id) = resolved_id {
        return Ok(Json(serde_json::json!({
            "local_conversation_id": q.local_conversation_id,
            "chat_conversation_id": chat_conversation_id,
            "match": match_kind.unwrap_or_else(|| "unknown".to_string())
        })));
    }

    Err((
        StatusCode::NOT_FOUND,
        Json(serde_json::json!({
            "error": "not_found",
            "local_conversation_id": q.local_conversation_id
        })),
    ))
}

pub async fn list_messages_handler(
    headers: HeaderMap,
    Path(conversation_id): Path<String>,
    Query(q): Query<ListMessagesQuery>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let token = extract_access_token(&headers).map_err(|s| (s, Json(serde_json::json!({ "error": "unauthorized" }))))?;
    chat_api_service::list_messages(&token, &conversation_id, q.limit.unwrap_or(50), q.before, q.after)
        .await
        .map(|v| Json(serde_json::to_value(v).unwrap()))
        .map_err(map_chat_error)
}

pub async fn send_message_handler(
    headers: HeaderMap,
    Path(conversation_id): Path<String>,
    Json(body): Json<SendMessageBody>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let token = extract_access_token(&headers).map_err(|s| (s, Json(serde_json::json!({ "error": "unauthorized" }))))?;
    chat_api_service::send_message(&token, &conversation_id, body.content)
        .await
        .map(|v| Json(serde_json::to_value(v).unwrap()))
        .map_err(map_chat_error)
}

// --- Reactions ---

pub async fn add_reaction_handler(
    headers: HeaderMap,
    Path((conversation_id, message_id)): Path<(String, String)>,
    Json(body): Json<ReactionBody>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let token = extract_access_token(&headers).map_err(|s| (s, Json(serde_json::json!({ "error": "unauthorized" }))))?;
    chat_api_service::add_reaction(&token, &conversation_id, &message_id, &body.emoji)
        .await
        .map(|v| Json(serde_json::to_value(v).unwrap()))
        .map_err(map_chat_error)
}

pub async fn remove_reaction_handler(
    headers: HeaderMap,
    Path((conversation_id, message_id, emoji)): Path<(String, String, String)>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let token = extract_access_token(&headers).map_err(|s| (s, Json(serde_json::json!({ "error": "unauthorized" }))))?;
    chat_api_service::remove_reaction(&token, &conversation_id, &message_id, &emoji)
        .await
        .map(|v| Json(serde_json::to_value(v).unwrap()))
        .map_err(map_chat_error)
}

// --- Polls ---

pub async fn create_poll_handler(
    headers: HeaderMap,
    Path((conversation_id, message_id)): Path<(String, String)>,
    Json(body): Json<PollCreateBody>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let token = extract_access_token(&headers).map_err(|s| (s, Json(serde_json::json!({ "error": "unauthorized" }))))?;
    chat_api_service::create_poll(&token, &conversation_id, &message_id, &body.question, body.options)
        .await
        .map(|v| Json(serde_json::to_value(v).unwrap()))
        .map_err(map_chat_error)
}

pub async fn vote_poll_handler(
    headers: HeaderMap,
    Path((conversation_id, poll_id)): Path<(String, String)>,
    Json(body): Json<PollVoteBody>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let token = extract_access_token(&headers).map_err(|s| (s, Json(serde_json::json!({ "error": "unauthorized" }))))?;
    chat_api_service::vote_on_poll(&token, &conversation_id, &poll_id, &body.option_id)
        .await
        .map(|v| Json(serde_json::to_value(v).unwrap()))
        .map_err(map_chat_error)
}

// --- Pinning ---

pub async fn pin_message_handler(
    headers: HeaderMap,
    Path((conversation_id, message_id)): Path<(String, String)>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let token = extract_access_token(&headers).map_err(|s| (s, Json(serde_json::json!({ "error": "unauthorized" }))))?;
    chat_api_service::pin_message(&token, &conversation_id, &message_id)
        .await
        .map(|v| Json(serde_json::to_value(v).unwrap()))
        .map_err(map_chat_error)
}

pub async fn unpin_message_handler(
    headers: HeaderMap,
    Path((conversation_id, message_id)): Path<(String, String)>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let token = extract_access_token(&headers).map_err(|s| (s, Json(serde_json::json!({ "error": "unauthorized" }))))?;
    chat_api_service::unpin_message(&token, &conversation_id, &message_id)
        .await
        .map(|v| Json(serde_json::to_value(v).unwrap()))
        .map_err(map_chat_error)
}

// --- Threading ---

pub async fn reply_to_message_handler(
    headers: HeaderMap,
    Path((conversation_id, message_id)): Path<(String, String)>,
    Json(body): Json<SendMessageBody>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let token = extract_access_token(&headers).map_err(|s| (s, Json(serde_json::json!({ "error": "unauthorized" }))))?;
    chat_api_service::reply_to_message(&token, &conversation_id, &message_id, body.content)
        .await
        .map(|v| Json(serde_json::to_value(v).unwrap()))
        .map_err(map_chat_error)
}

// --- Scheduling ---

pub async fn schedule_message_handler(
    headers: HeaderMap,
    Path(conversation_id): Path<String>,
    Json(body): Json<ScheduleMessageBody>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let token = extract_access_token(&headers).map_err(|s| (s, Json(serde_json::json!({ "error": "unauthorized" }))))?;
    chat_api_service::schedule_message(&token, &conversation_id, body.content, body.scheduled_for)
        .await
        .map(|v| Json(serde_json::to_value(v).unwrap()))
        .map_err(map_chat_error)
}

// --- Utility ---

pub async fn get_unread_count_handler(
    headers: HeaderMap,
    Path(conversation_id): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let token = extract_access_token(&headers).map_err(|s| (s, Json(serde_json::json!({ "error": "unauthorized" }))))?;
    chat_api_service::get_unread_count(&token, &conversation_id)
        .await
        .map(|v| Json(serde_json::to_value(v).unwrap()))
        .map_err(map_chat_error)
}