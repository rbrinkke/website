use sqlx::SqlitePool;

#[derive(Debug, sqlx::FromRow, Clone)]
pub struct ActivitySummaryRow {
    pub activity_id: String,
    pub title: String,
    pub description: Option<String>,
    pub scheduled_at: String,
    pub duration_minutes: Option<i64>,
    pub max_participants: i64,
    pub current_participants_count: i64,
    pub waitlist_count: i64,
    pub status: String,
    pub is_joined: i64,
    pub am_on_waitlist: i64,
    pub city: Option<String>,
    pub location: Option<String>,
    pub organizer: String,
    pub tags: Option<String>,
    pub category: Option<String>,
    pub main_photo_asset_id: Option<String>,
    pub waitlist_enabled: i64,
}

const SQL_LOAD_ACTIVITY_SUMMARY: &str = r#"
SELECT
  a.activity_id,
  a.title,
  a.description,
  a.scheduled_at,
  a.duration_minutes,
  a.max_participants,
  a.current_participants_count,
  a.waitlist_count,
  a.status,
  a.is_joined,
  a.am_on_waitlist,
  a.city,
  a.location,
  a.organizer,
  a.tags,
  a.category,
  a.main_photo_asset_id,
  COALESCE(s.waitlist_enabled, 1) AS waitlist_enabled
FROM activities a
LEFT JOIN activity_settings s
  ON s.activity_id = a.activity_id
WHERE a.activity_id = ?1
  AND a.is_deleted = 0
LIMIT 1
"#;

pub async fn load_activity_summary(
    pool: &SqlitePool,
    activity_id: &str,
) -> sqlx::Result<Option<ActivitySummaryRow>> {
    sqlx::query_as::<_, ActivitySummaryRow>(SQL_LOAD_ACTIVITY_SUMMARY)
        .bind(activity_id)
        .fetch_optional(pool)
        .await
}
