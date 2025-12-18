use sqlx::{Connection, SqlitePool};

use crate::database::chat_cache_repo;
use crate::database::chat_conversations_repo;
use crate::models::ChatConversationRow;

pub async fn load_chat_inbox(pool: &SqlitePool) -> sqlx::Result<Vec<ChatConversationRow>> {
    chat_conversations_repo::list_chat_conversations(pool).await
}

pub async fn load_chat_conversation(
    pool: &SqlitePool,
    conversation_id: &str,
) -> sqlx::Result<Option<ChatConversationRow>> {
    chat_conversations_repo::get_chat_conversation_by_id(pool, conversation_id).await
}

pub fn chat_cache_path() -> String {
    std::env::var("CHAT_CACHE_DB_PATH").unwrap_or_else(|_| "chat_cache/chat_cache.db".to_string())
}

pub async fn load_chat_cache_preview(
    conversation_id: &str,
) -> sqlx::Result<Option<chat_cache_repo::ChatCacheConversationPreview>> {
    let path = chat_cache_path();
    if !std::path::Path::new(&path).exists() {
        return Ok(None);
    }

    let mut conn = sqlx::SqliteConnection::connect(&format!("sqlite://{}", path)).await?;
    chat_cache_repo::get_conversation_preview(&mut conn, conversation_id).await
}

pub async fn load_chat_cache_messages(
    conversation_id: &str,
    limit: i64,
) -> sqlx::Result<Vec<chat_cache_repo::ChatCacheMessage>> {
    let path = chat_cache_path();
    if !std::path::Path::new(&path).exists() {
        return Ok(vec![]);
    }

    let mut conn = sqlx::SqliteConnection::connect(&format!("sqlite://{}", path)).await?;
    chat_cache_repo::list_messages(&mut conn, conversation_id, limit).await
}

pub async fn load_chat_cache_previews(
    limit: i64,
) -> sqlx::Result<Vec<chat_cache_repo::ChatCacheConversationPreview>> {
    let path = chat_cache_path();
    if !std::path::Path::new(&path).exists() {
        return Ok(vec![]);
    }

    let mut conn = sqlx::SqliteConnection::connect(&format!("sqlite://{}", path)).await?;
    chat_cache_repo::list_conversation_previews(&mut conn, limit).await
}
