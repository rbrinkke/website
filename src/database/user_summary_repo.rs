use sqlx::SqlitePool;

#[derive(Debug, sqlx::FromRow, Clone)]
pub struct UserSummaryRow {
    pub user_id: String,
    pub name: Option<String>,
    pub profile_description: Option<String>,
    pub age: Option<i64>,
    pub gender: Option<String>,
    pub city: Option<String>,
    pub country: Option<String>,
    pub is_verified: Option<i64>,
    pub interests: Option<String>,
    pub subscription_level: Option<String>,
    pub last_seen_at: Option<String>,
    pub friendship_status: Option<String>,
    pub initiated_by_me: Option<i64>,
    pub chat_conversation_id: Option<String>,
}

const SQL_LOAD_USER_SUMMARY: &str = r#"
SELECT
  u.user_id,
  u.name,
  u.profile_description,
  u.age,
  u.gender,
  u.city,
  u.country,
  u.is_verified,
  u.interests,
  u.subscription_level,
  u.last_seen_at,
  f.status AS friendship_status,
  f.initiated_by_me AS initiated_by_me,
  c.conversation_id AS chat_conversation_id
FROM users u
LEFT JOIN friends f ON (
  (f.friendship_id = ?1 || ':' || u.user_id OR f.friendship_id = u.user_id || ':' || ?1)
  AND (f.is_deleted = 0 OR f.is_deleted IS NULL)
)
LEFT JOIN chat_conversations c ON (
  c.other_user_id = u.user_id
  AND c.chat_context = 'private'
  AND c.is_deleted = 0
)
WHERE u.user_id = ?2
  AND (u.is_deleted = 0 OR u.is_deleted IS NULL)
LIMIT 1
"#;

pub async fn load_user_summary(
    pool: &SqlitePool,
    auth_user_id: &str,
    user_id: &str,
) -> sqlx::Result<Option<UserSummaryRow>> {
    sqlx::query_as::<_, UserSummaryRow>(SQL_LOAD_USER_SUMMARY)
        .bind(auth_user_id)
        .bind(user_id)
        .fetch_optional(pool)
        .await
}
