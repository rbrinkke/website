use sqlx::SqlitePool;

#[derive(Debug, sqlx::FromRow)]
pub struct ActivityGeoCandidateRow {
    pub activity_id: String,
    pub title: String,
    pub location: String,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
}

const SQL_LIST_ACTIVITIES_MISSING_GEO: &str = r#"
SELECT
  activity_id,
  title,
  location,
  latitude,
  longitude
FROM activities
WHERE is_deleted = 0
  AND (latitude IS NULL OR longitude IS NULL)
  AND location IS NOT NULL
  AND location != ''
ORDER BY scheduled_at ASC
LIMIT ?
"#;

pub async fn list_activities_missing_geo(
    pool: &SqlitePool,
    limit: i64,
) -> sqlx::Result<Vec<ActivityGeoCandidateRow>> {
    sqlx::query_as::<_, ActivityGeoCandidateRow>(SQL_LIST_ACTIVITIES_MISSING_GEO)
        .bind(limit)
        .fetch_all(pool)
        .await
}

const SQL_UPDATE_ACTIVITY_GEO: &str = r#"
UPDATE activities
SET latitude = ?, longitude = ?
WHERE activity_id = ?
"#;

pub async fn update_activity_geo(
    pool: &SqlitePool,
    activity_id: &str,
    latitude: f64,
    longitude: f64,
) -> sqlx::Result<u64> {
    let res = sqlx::query(SQL_UPDATE_ACTIVITY_GEO)
        .bind(latitude)
        .bind(longitude)
        .bind(activity_id)
        .execute(pool)
        .await?;
    Ok(res.rows_affected())
}
