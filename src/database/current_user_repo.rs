use sqlx::SqlitePool;

use crate::models::CurrentUserRow;

pub const SQL_LOAD_CURRENT_USER_ID: &str = r#"
SELECT user_id
FROM current_user
LIMIT 1
"#;

pub async fn load_current_user_id(pool: &SqlitePool) -> sqlx::Result<Option<String>> {
    let row = sqlx::query_as::<_, CurrentUserRow>(SQL_LOAD_CURRENT_USER_ID)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(|r| r.user_id))
}
