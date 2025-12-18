use axum::{
    extract::{Path, Query},
    http::{header, HeaderMap, StatusCode},
    Json,
};
use serde::Deserialize;
use serde_json::Value;

use crate::services::chat_api_service;

fn extract_access_token(headers: &HeaderMap) -> Result<String, StatusCode> {
    let cookies = headers
        .get(header::COOKIE)
        .and_then(|hv| hv.to_str().ok())
        .unwrap_or("");

    tracing::info!(cookies_raw = %cookies, "extract_access_token: parsing cookies");

    let token = cookies
        .split("; ")
        .find_map(|cookie| cookie.strip_prefix("access_token=").map(|t| t.to_string()));

    match &token {
        Some(t) => tracing::info!(token_len = t.len(), "extract_access_token: found token"),
        None => tracing::warn!("extract_access_token: NO access_token cookie found"),
    }

    token.ok_or(StatusCode::UNAUTHORIZED)
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
    /// Local snapshot conversation id (activity_id or private_chat_id)
    pub local_conversation_id: String,
}

pub async fn ws_ticket_handler(
    headers: HeaderMap,
    Json(body): Json<WsTicketBody>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let token = extract_access_token(&headers)
        .map_err(|s| (s, Json(serde_json::json!({ "error": "unauthorized" }))))?;
    chat_api_service::ws_ticket(&token, &body.conversation_id)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::warn!(status = %e.status, body = ?e.body, "chat_api_ws_ticket_failed");
            (
                e.status,
                Json(
                    e.body
                        .unwrap_or_else(|| serde_json::json!({ "error": "bad_gateway" })),
                ),
            )
        })
}

pub async fn health_handler() -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    match chat_api_service::health().await {
        Ok(v) => Ok(Json(v)),
        Err(e) => Err((
            e.status,
            Json(
                e.body
                    .unwrap_or_else(|| serde_json::json!({ "error": "bad_gateway" })),
            ),
        )),
    }
}

pub async fn resolve_conversation_handler(
    headers: HeaderMap,
    Query(q): Query<ResolveChatQuery>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    tracing::info!(local_conversation_id = %q.local_conversation_id, "resolve_conversation: START");

    let token = extract_access_token(&headers)
        .map_err(|s| {
            tracing::warn!("resolve_conversation: unauthorized - no token");
            (s, Json(serde_json::json!({ "error": "unauthorized" })))
        })?;

    tracing::info!("resolve_conversation: got token, calling chat-api");

    // We only have a local snapshot id. In chat-api, activity chats typically store that as `external_id`,
    // while the primary `id` is the chat db conversation UUID.
    let conversations = chat_api_service::list_conversations(&token, 50, None)
        .await
        .map_err(|e| {
            tracing::warn!(status = %e.status, body = ?e.body, "chat_api_list_conversations_failed");
            (
                e.status,
                Json(e.body.unwrap_or_else(|| serde_json::json!({ "error": "bad_gateway" }))),
            )
        })?;

    let list = conversations
        .get("conversations")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    // Prefer direct match on id; fallback to match on external_id.
    let mut resolved_id: Option<String> = None;
    let mut match_kind: Option<String> = None;
    for c in &list {
        let id = c.get("id").and_then(|v| v.as_str());
        if id == Some(q.local_conversation_id.as_str()) {
            resolved_id = id.map(|s| s.to_string());
            match_kind = Some("id".to_string());
            break;
        }
    }
    if resolved_id.is_none() {
        for c in &list {
            let external_id = c.get("external_id").and_then(|v| v.as_str());
            if external_id == Some(q.local_conversation_id.as_str()) {
                resolved_id = c.get("id").and_then(|v| v.as_str()).map(|s| s.to_string());
                match_kind = Some("external_id".to_string());
                break;
            }
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
            "local_conversation_id": q.local_conversation_id,
            "hint": "Conversation not present in chat-api inbox; maybe not joined/created yet."
        })),
    ))
}

pub async fn list_messages_handler(
    headers: HeaderMap,
    Path(conversation_id): Path<String>,
    Query(q): Query<ListMessagesQuery>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let token = extract_access_token(&headers)
        .map_err(|s| (s, Json(serde_json::json!({ "error": "unauthorized" }))))?;

    chat_api_service::list_messages(
        &token,
        &conversation_id,
        q.limit.unwrap_or(50),
        q.before,
        q.after,
    )
    .await
    .map(Json)
    .map_err(|e| {
        tracing::warn!(status = %e.status, body = ?e.body, "chat_api_list_messages_failed");
        (
            e.status,
            Json(
                e.body
                    .unwrap_or_else(|| serde_json::json!({ "error": "bad_gateway" })),
            ),
        )
    })
}

pub async fn send_message_handler(
    headers: HeaderMap,
    Path(conversation_id): Path<String>,
    Json(body): Json<SendMessageBody>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let token = extract_access_token(&headers)
        .map_err(|s| (s, Json(serde_json::json!({ "error": "unauthorized" }))))?;

    if body.content.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "empty_message" })),
        ));
    }

    chat_api_service::send_message(&token, &conversation_id, body.content)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::warn!(status = %e.status, body = ?e.body, "chat_api_send_message_failed");
            (
                e.status,
                Json(
                    e.body
                        .unwrap_or_else(|| serde_json::json!({ "error": "bad_gateway" })),
                ),
            )
        })
}
