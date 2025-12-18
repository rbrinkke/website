use sqlx::SqlitePool;

#[derive(Debug, sqlx::FromRow, Clone)]
pub struct InterestRow {
    pub name: String,
    pub emoji: Option<String>,
    pub category_name: Option<String>,
    pub sort_order: i64,
}

const SQL_LIST_ACTIVE_INTERESTS: &str = r#"
SELECT
  name,
  emoji,
  category_name,
  sort_order
FROM interests
WHERE is_deleted = 0
  AND is_active = 1
  AND TRIM(COALESCE(name, '')) != ''
ORDER BY
  COALESCE(NULLIF(TRIM(category_name), ''), 'zzz') ASC,
  sort_order ASC,
  name ASC
LIMIT ?
"#;

pub async fn list_active(pool: &SqlitePool, limit: i64) -> sqlx::Result<Vec<InterestRow>> {
    sqlx::query_as::<_, InterestRow>(SQL_LIST_ACTIVE_INTERESTS)
        .bind(limit)
        .fetch_all(pool)
        .await
}
