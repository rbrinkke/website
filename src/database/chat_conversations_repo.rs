use sqlx::SqlitePool;

use crate::models::ChatConversationRow;

pub const SQL_LIST_CHAT_CONVERSATIONS: &str = r#"
SELECT
  conversation_id,
  chat_context,
  relationship_status,
  title,
  subtitle,
  image_asset_id,
  target_id,
  effective_mask,
  chat_status,
  is_initiator,
  block_direction,
  mute_expires_at,
  participant_role,
  other_user_id,
  other_user_name,
  other_user_photo_asset_id,
  other_user_username,
  other_user_is_verified,
  activity_status,
  activity_scheduled_at,
  activity_city,
  activity_location_name,
  activity_main_photo_asset_id,
  row_hash,
  changed_at,
  is_deleted
FROM chat_conversations
WHERE is_deleted = 0
ORDER BY changed_at DESC
"#;

pub async fn list_chat_conversations(pool: &SqlitePool) -> sqlx::Result<Vec<ChatConversationRow>> {
    sqlx::query_as::<_, ChatConversationRow>(SQL_LIST_CHAT_CONVERSATIONS)
        .fetch_all(pool)
        .await
}

pub const SQL_GET_CHAT_CONVERSATION_BY_ID: &str = r#"
SELECT
  conversation_id,
  chat_context,
  relationship_status,
  title,
  subtitle,
  image_asset_id,
  target_id,
  effective_mask,
  chat_status,
  is_initiator,
  block_direction,
  mute_expires_at,
  participant_role,
  other_user_id,
  other_user_name,
  other_user_photo_asset_id,
  other_user_username,
  other_user_is_verified,
  activity_status,
  activity_scheduled_at,
  activity_city,
  activity_location_name,
  activity_main_photo_asset_id,
  row_hash,
  changed_at,
  is_deleted
FROM chat_conversations
WHERE is_deleted = 0
  AND conversation_id = ?1
LIMIT 1
"#;

pub async fn get_chat_conversation_by_id(
    pool: &SqlitePool,
    conversation_id: &str,
) -> sqlx::Result<Option<ChatConversationRow>> {
    sqlx::query_as::<_, ChatConversationRow>(SQL_GET_CHAT_CONVERSATION_BY_ID)
        .bind(conversation_id)
        .fetch_optional(pool)
        .await
}
