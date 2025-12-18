use sqlx::SqlitePool;

use crate::models::UsersRow;

pub const SQL_LOAD_USER_PROFILE: &str = r#"
SELECT
    name,
    profile_description,
    age,
    gender,
    city,
    country,
    main_photo_url,
    profile_photos_extra,
    is_verified,
    interests,
    subscription_level,
    activities_created_count,
    activities_attended_count,
    last_seen_at
FROM users
WHERE user_id = ?1
  AND (is_deleted = 0 OR is_deleted IS NULL)
LIMIT 1
"#;

pub async fn load_user_profile(pool: &SqlitePool, user_id: &str) -> sqlx::Result<Option<UsersRow>> {
    sqlx::query_as::<_, UsersRow>(SQL_LOAD_USER_PROFILE)
        .bind(user_id)
        .fetch_optional(pool)
        .await
}
