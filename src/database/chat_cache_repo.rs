use sqlx::{Row, SqliteConnection};

#[derive(Debug, Clone)]
pub struct ChatCacheConversationPreview {
    pub conversation_id: String,
    pub last_message_preview: Option<String>,
    pub last_message_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ChatCacheMessage {
    pub conversation_id: String,
    pub message_id: String,
    pub created_at: String,
    pub sender_id: String,
    pub message_type: String,
    pub content: Option<String>,
    pub metadata_json: String,
    pub status: Option<String>,
    pub is_deleted: i64,
    pub edited_at: Option<String>,
}

pub async fn list_conversation_previews(
    conn: &mut SqliteConnection,
    limit: i64,
) -> sqlx::Result<Vec<ChatCacheConversationPreview>> {
    let rows = sqlx::query(
        r#"
SELECT
  conversation_id,
  last_message_preview,
  last_message_at
FROM conversations
ORDER BY updated_at DESC
LIMIT ?1
        "#,
    )
    .bind(limit)
    .fetch_all(conn)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| ChatCacheConversationPreview {
            conversation_id: row.get("conversation_id"),
            last_message_preview: row.get("last_message_preview"),
            last_message_at: row.get("last_message_at"),
        })
        .collect())
}

pub async fn get_conversation_preview(
    conn: &mut SqliteConnection,
    conversation_id: &str,
) -> sqlx::Result<Option<ChatCacheConversationPreview>> {
    let row = sqlx::query(
        r#"
SELECT
  conversation_id,
  last_message_preview,
  last_message_at
FROM conversations
WHERE conversation_id = ?1
LIMIT 1
        "#,
    )
    .bind(conversation_id)
    .fetch_optional(conn)
    .await?;

    Ok(row.map(|row| ChatCacheConversationPreview {
        conversation_id: row.get("conversation_id"),
        last_message_preview: row.get("last_message_preview"),
        last_message_at: row.get("last_message_at"),
    }))
}

pub async fn list_messages(
    conn: &mut SqliteConnection,
    conversation_id: &str,
    limit: i64,
) -> sqlx::Result<Vec<ChatCacheMessage>> {
    let rows = sqlx::query(
        r#"
SELECT
  conversation_id,
  message_id,
  created_at,
  sender_id,
  message_type,
  content,
  metadata_json,
  status,
  is_deleted,
  edited_at
FROM messages
WHERE conversation_id = ?1
ORDER BY created_at ASC
LIMIT ?2
        "#,
    )
    .bind(conversation_id)
    .bind(limit)
    .fetch_all(conn)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| ChatCacheMessage {
            conversation_id: row.get("conversation_id"),
            message_id: row.get("message_id"),
            created_at: row.get("created_at"),
            sender_id: row.get("sender_id"),
            message_type: row.get("message_type"),
            content: row.get("content"),
            metadata_json: row.get("metadata_json"),
            status: row.get("status"),
            is_deleted: row.get("is_deleted"),
            edited_at: row.get("edited_at"),
        })
        .collect())
}
