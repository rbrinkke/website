#[allow(dead_code)]
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct FriendsRow {
    pub friendship_id: String,
    pub status: String,
    pub is_deleted: Option<i64>,
}
