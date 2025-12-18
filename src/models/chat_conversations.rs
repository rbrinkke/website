use sqlx::FromRow;

#[derive(Debug, Clone, FromRow)]
pub struct ChatConversationRow {
    pub conversation_id: String,
    pub chat_context: String,
    pub relationship_status: String,

    pub title: Option<String>,
    pub subtitle: Option<String>,
    pub image_asset_id: Option<String>,
    pub target_id: Option<String>,

    pub effective_mask: Option<i64>,
    pub chat_status: Option<String>,
    pub is_initiator: Option<i64>,
    pub block_direction: Option<String>,
    pub mute_expires_at: Option<String>,
    pub participant_role: Option<String>,
    pub other_user_id: Option<String>,

    pub other_user_name: Option<String>,
    pub other_user_photo_asset_id: Option<String>,
    pub other_user_username: Option<String>,
    pub other_user_is_verified: Option<i64>,

    pub activity_status: Option<String>,
    pub activity_scheduled_at: Option<String>,
    pub activity_city: Option<String>,
    pub activity_location_name: Option<String>,
    pub activity_main_photo_asset_id: Option<String>,

    pub row_hash: String,
    pub changed_at: String,
    pub is_deleted: i64,
}
