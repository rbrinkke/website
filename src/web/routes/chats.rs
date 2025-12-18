use askama::Template;
use axum::{
    extract::{Path, State},
    response::Html,
    Extension,
};
use sqlx::SqlitePool;

use crate::services::chat_inbox_service;
use crate::web::middleware::auth::AuthenticatedUser;

fn format_last_message_at(ts: Option<String>) -> Option<String> {
    let ts = ts?;
    let ts = ts.trim();
    if ts.is_empty() {
        return None;
    }

    // Common cases:
    // - RFC3339-ish: 2025-12-17T11:33:44Z -> 11:33
    // - SQLite-ish:  2025-12-17 11:33:44 -> 11:33
    if let Some(t_index) = ts.find('T') {
        let time = ts.get(t_index + 1..)?;
        if time.len() >= 5 {
            return Some(time[..5].to_string());
        }
    }
    if let Some(space_index) = ts.find(' ') {
        let time = ts.get(space_index + 1..)?;
        if time.len() >= 5 {
            return Some(time[..5].to_string());
        }
    }
    if ts.len() >= 5 {
        return Some(ts[..5].to_string());
    }
    Some(ts.to_string())
}

fn normalize_preview(preview: Option<String>) -> Option<String> {
    let mut s = preview?;
    s = s.replace('\n', " ");
    s = s.replace('\r', " ");
    let s = s.trim().to_string();
    if s.is_empty() {
        return None;
    }
    const MAX: usize = 72;
    if s.chars().count() <= MAX {
        return Some(s);
    }
    let mut out = String::new();
    for (idx, ch) in s.chars().enumerate() {
        if idx >= MAX {
            break;
        }
        out.push(ch);
    }
    out.push('…');
    Some(out)
}

fn normalize_preview_no_truncate(preview: Option<String>) -> Option<String> {
    let mut s = preview?;
    s = s.replace('\n', " ");
    s = s.replace('\r', " ");
    let s = s.trim().to_string();
    if s.is_empty() {
        return None;
    }
    Some(s)
}

#[derive(Debug, Clone)]
struct ChatInboxItemView {
    conversation: crate::models::ChatConversationRow,
    last_message_preview: Option<String>,
    last_message_at: Option<String>,
}

#[derive(Template)]
#[template(path = "chats.html")]
struct ChatsTemplate {
    conversations: Vec<ChatInboxItemView>,
    build_id: String,
}

#[derive(Template)]
#[template(path = "error.html")]
struct ErrorTemplate {
    message: String,
}

pub async fn chats_handler(State(pool): State<SqlitePool>) -> Html<String> {
    match chat_inbox_service::load_chat_inbox(&pool).await {
        Ok(conversations) => {
            let previews = chat_inbox_service::load_chat_cache_previews(200)
                .await
                .unwrap_or_default();
            let preview_map: std::collections::HashMap<String, (Option<String>, Option<String>)> =
                previews
                    .into_iter()
                    .map(|p| {
                        (
                            p.conversation_id,
                            (p.last_message_preview, p.last_message_at),
                        )
                    })
                    .collect();

            let items = conversations
                .into_iter()
                .map(|c| {
                    let (last_message_preview, last_message_at) = preview_map
                        .get(&c.conversation_id)
                        .cloned()
                        .unwrap_or((None, None));
                    ChatInboxItemView {
                        conversation: c,
                        last_message_preview: normalize_preview(last_message_preview),
                        last_message_at: format_last_message_at(last_message_at),
                    }
                })
                .collect();

            let template = ChatsTemplate {
                conversations: items,
                build_id: std::env::var("GOAMET_BUILD_ID").unwrap_or_else(|_| "dev".to_string()),
            };
            Html(template.render().unwrap())
        }
        Err(err) => {
            tracing::error!(error = %err, "chats_handler_failed");
            let template = ErrorTemplate {
                message: "Kon chats niet laden".to_string(),
            };
            Html(
                template
                    .render()
                    .unwrap_or_else(|_| "Kon chats niet laden".to_string()),
            )
        }
    }
}

#[derive(Template)]
#[template(path = "chat_detail.html")]
struct ChatDetailTemplate {
    current_user_id: String,
    conversation: crate::models::ChatConversationRow,
    messages: Vec<crate::database::chat_cache_repo::ChatCacheMessage>,
    preview_last_message_at: Option<String>,
    preview_last_message_preview: Option<String>,
    build_id: String,
}

pub async fn chat_detail_handler(
    Extension(auth_user): Extension<AuthenticatedUser>,
    State(pool): State<SqlitePool>,
    Path(conversation_id): Path<String>,
) -> Html<String> {
    match chat_inbox_service::load_chat_conversation(&pool, &conversation_id).await {
        Ok(Some(conversation)) => {
            let preview = chat_inbox_service::load_chat_cache_preview(&conversation_id)
                .await
                .ok()
                .flatten();
            let messages = chat_inbox_service::load_chat_cache_messages(&conversation_id, 300)
                .await
                .unwrap_or_default();
            let preview_last_message_at = preview.as_ref().and_then(|p| p.last_message_at.clone());
            let preview_last_message_preview = normalize_preview_no_truncate(
                preview
                    .as_ref()
                    .and_then(|p| p.last_message_preview.clone()),
            )
            .or_else(|| {
                messages.iter().rev().find_map(|m| {
                    if m.is_deleted == 1 {
                        return None;
                    }
                    if m.message_type != "text" {
                        return None;
                    }
                    let content = m.content.as_ref()?.trim();
                    if content.is_empty() {
                        return None;
                    }
                    let mut s = content.to_string();
                    if m.sender_id == auth_user.id {
                        s = format!("Jij: {}", s);
                    }
                    normalize_preview_no_truncate(Some(s))
                })
            });

            let template = ChatDetailTemplate {
                current_user_id: auth_user.id,
                conversation,
                messages,
                preview_last_message_at,
                preview_last_message_preview,
                build_id: std::env::var("GOAMET_BUILD_ID").unwrap_or_else(|_| "dev".to_string()),
            };
            Html(template.render().unwrap())
        }
        Ok(None) => {
            let template = ErrorTemplate {
                message: "Chat niet gevonden (sync nog niet klaar?)".to_string(),
            };
            Html(
                template
                    .render()
                    .unwrap_or_else(|_| "Chat niet gevonden".to_string()),
            )
        }
        Err(err) => {
            tracing::error!(error = %err, conversation_id = %conversation_id, "chat_detail_handler_failed");
            let template = ErrorTemplate {
                message: "Kon chat niet laden".to_string(),
            };
            Html(
                template
                    .render()
                    .unwrap_or_else(|_| "Kon chat niet laden".to_string()),
            )
        }
    }
}
