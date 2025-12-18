use sqlx::SqlitePool;

#[derive(Debug, sqlx::FromRow, Clone)]
pub struct ActivityFeedRow {
    pub activity_id: String,
    pub title: String,
    pub scheduled_at: String,
    pub status: String,
    pub is_joined: i64,
    pub city: Option<String>,
    pub venue_name: Option<String>,
    pub organizer_name: Option<String>,
    pub organizer_user_id: Option<String>,
    pub organizer_photo_asset_id: Option<String>,
    pub main_photo_asset_id: Option<String>,
    pub tags: Option<String>,
    pub max_participants: i64,
    pub current_participants_count: i64,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub avatar_urls: Option<String>,
    pub participants_preview_json: Option<String>,
    pub waitlist_enabled: i64,
    pub is_past: i64,
}

const SQL_LIST_UPCOMING: &str = r#"
SELECT
  a.activity_id,
  a.title,
  a.scheduled_at,
  a.status,
  a.is_joined,
  a.city,
  json_extract(a.location, '$.venue_name') AS venue_name,
  a.primary_organizer_user_id AS organizer_user_id,
  a.primary_organizer_name AS organizer_name,
  a.primary_organizer_photo_asset_id AS organizer_photo_asset_id,
  a.main_photo_asset_id,
  a.tags,
  a.max_participants,
  a.current_participants_count,
  a.latitude,
  a.longitude,
  (
    SELECT group_concat(photo_url, '\n')
    FROM (
      SELECT ap.photo_url
      FROM activity_participants ap
      WHERE ap.activity_id = a.activity_id
        AND ap.is_deleted = 0
        AND ap.photo_url IS NOT NULL
        AND ap.photo_url != ''
      ORDER BY ap.joined_at ASC
      LIMIT 8
    )
  ) AS avatar_urls,
  (
    SELECT json_group_array(
      json_object(
        'user_id', user_id,
        'photo_url', photo_url,
        'name', name,
        'is_verified', is_verified,
        'is_friend', is_friend
      )
    )
    FROM (
      SELECT
        ap.photo_url AS photo_url,
        ap.user_id AS user_id,
        COALESCE(NULLIF(TRIM(ap.name), ''), NULLIF(TRIM(u.name), ''), 'deelnemer') AS name,
        COALESCE(u.is_verified, 0) AS is_verified,
        CASE WHEN f.friendship_id IS NOT NULL THEN 1 ELSE 0 END AS is_friend
      FROM activity_participants ap
      LEFT JOIN users u ON u.user_id = ap.user_id
      LEFT JOIN friends f ON (
        (f.friendship_id = ? || ':' || ap.user_id OR f.friendship_id = ap.user_id || ':' || ?)
        AND f.status = 'accepted'
        AND (f.is_deleted = 0 OR f.is_deleted IS NULL)
      )
      WHERE ap.activity_id = a.activity_id
        AND ap.is_deleted = 0
        AND ap.photo_url IS NOT NULL
        AND ap.photo_url != ''
      ORDER BY ap.joined_at ASC
      LIMIT 30
    )
  ) AS participants_preview_json,
  COALESCE(s.waitlist_enabled, 1) AS waitlist_enabled,
  CASE WHEN datetime(a.scheduled_at) <= datetime('now') THEN 1 ELSE 0 END AS is_past
FROM activities a
LEFT JOIN activity_settings s
  ON s.activity_id = a.activity_id
WHERE a.is_deleted = 0
  AND a.is_joined = 1
  AND datetime(a.scheduled_at) > datetime('now')
  AND (
    ? = ''
    OR lower(a.title) LIKE ?
    OR lower(COALESCE(a.city, '')) LIKE ?
    OR lower(COALESCE(json_extract(a.location, '$.venue_name'), '')) LIKE ?
  )
  AND (
    ? IS NULL
    OR (
      a.latitude BETWEEN ? AND ?
      AND a.longitude BETWEEN ? AND ?
    )
  )
ORDER BY datetime(a.scheduled_at) ASC
LIMIT ?
"#;

pub async fn list_upcoming(
    pool: &SqlitePool,
    auth_user_id: &str,
    q_like: &str,
    bbox: Option<(f64, f64, f64, f64)>,
    limit: i64,
) -> sqlx::Result<Vec<ActivityFeedRow>> {
    let (min_lat, max_lat, min_lon, max_lon) = bbox
        .map(|v| (Some(v.0), Some(v.1), Some(v.2), Some(v.3)))
        .unwrap_or((None, None, None, None));

    sqlx::query_as::<_, ActivityFeedRow>(SQL_LIST_UPCOMING)
        .bind(auth_user_id)
        .bind(auth_user_id)
        .bind(q_like)
        .bind(q_like)
        .bind(q_like)
        .bind(q_like)
        .bind(min_lat)
        .bind(min_lat)
        .bind(max_lat)
        .bind(min_lon)
        .bind(max_lon)
        .bind(limit)
        .fetch_all(pool)
        .await
}

const SQL_LIST_DISCOVER: &str = r#"
SELECT
  a.activity_id,
  a.title,
  a.scheduled_at,
  a.status,
  a.is_joined,
  a.city,
  json_extract(a.location, '$.venue_name') AS venue_name,
  a.primary_organizer_user_id AS organizer_user_id,
  a.primary_organizer_name AS organizer_name,
  a.primary_organizer_photo_asset_id AS organizer_photo_asset_id,
  a.main_photo_asset_id,
  a.tags,
  a.max_participants,
  a.current_participants_count,
  a.latitude,
  a.longitude,
  (
    SELECT group_concat(photo_url, '\n')
    FROM (
      SELECT ap.photo_url
      FROM activity_participants ap
      WHERE ap.activity_id = a.activity_id
        AND ap.is_deleted = 0
        AND ap.photo_url IS NOT NULL
        AND ap.photo_url != ''
      ORDER BY ap.joined_at ASC
      LIMIT 8
    )
  ) AS avatar_urls,
  (
    SELECT json_group_array(
      json_object(
        'user_id', user_id,
        'photo_url', photo_url,
        'name', name,
        'is_verified', is_verified,
        'is_friend', is_friend
      )
    )
    FROM (
      SELECT
        ap.photo_url AS photo_url,
        ap.user_id AS user_id,
        COALESCE(NULLIF(TRIM(ap.name), ''), NULLIF(TRIM(u.name), ''), 'deelnemer') AS name,
        COALESCE(u.is_verified, 0) AS is_verified,
        CASE WHEN f.friendship_id IS NOT NULL THEN 1 ELSE 0 END AS is_friend
      FROM activity_participants ap
      LEFT JOIN users u ON u.user_id = ap.user_id
      LEFT JOIN friends f ON (
        (f.friendship_id = ? || ':' || ap.user_id OR f.friendship_id = ap.user_id || ':' || ?)
        AND f.status = 'accepted'
        AND (f.is_deleted = 0 OR f.is_deleted IS NULL)
      )
      WHERE ap.activity_id = a.activity_id
        AND ap.is_deleted = 0
        AND ap.photo_url IS NOT NULL
        AND ap.photo_url != ''
      ORDER BY ap.joined_at ASC
      LIMIT 30
    )
  ) AS participants_preview_json,
  COALESCE(s.waitlist_enabled, 1) AS waitlist_enabled,
  CASE WHEN datetime(a.scheduled_at) <= datetime('now') THEN 1 ELSE 0 END AS is_past
FROM activities a
LEFT JOIN activity_settings s
  ON s.activity_id = a.activity_id
WHERE a.is_deleted = 0
  AND a.is_joined = 0
  AND a.status = 'published'
  AND datetime(a.scheduled_at) > datetime('now')
  AND (
    ? = ''
    OR lower(a.title) LIKE ?
    OR lower(COALESCE(a.city, '')) LIKE ?
    OR lower(COALESCE(json_extract(a.location, '$.venue_name'), '')) LIKE ?
  )
  AND (
    ? IS NULL
    OR (
      a.latitude BETWEEN ? AND ?
      AND a.longitude BETWEEN ? AND ?
    )
  )
ORDER BY datetime(a.scheduled_at) ASC
LIMIT ?
"#;

pub async fn list_discover(
    pool: &SqlitePool,
    auth_user_id: &str,
    q_like: &str,
    bbox: Option<(f64, f64, f64, f64)>,
    limit: i64,
) -> sqlx::Result<Vec<ActivityFeedRow>> {
    let (min_lat, max_lat, min_lon, max_lon) = bbox
        .map(|v| (Some(v.0), Some(v.1), Some(v.2), Some(v.3)))
        .unwrap_or((None, None, None, None));

    sqlx::query_as::<_, ActivityFeedRow>(SQL_LIST_DISCOVER)
        .bind(auth_user_id)
        .bind(auth_user_id)
        .bind(q_like)
        .bind(q_like)
        .bind(q_like)
        .bind(q_like)
        .bind(min_lat)
        .bind(min_lat)
        .bind(max_lat)
        .bind(min_lon)
        .bind(max_lon)
        .bind(limit)
        .fetch_all(pool)
        .await
}

const SQL_LIST_HISTORY: &str = r#"
SELECT
  a.activity_id,
  a.title,
  a.scheduled_at,
  a.status,
  a.is_joined,
  a.city,
  json_extract(a.location, '$.venue_name') AS venue_name,
  a.primary_organizer_user_id AS organizer_user_id,
  a.primary_organizer_name AS organizer_name,
  a.primary_organizer_photo_asset_id AS organizer_photo_asset_id,
  a.main_photo_asset_id,
  a.tags,
  a.max_participants,
  a.current_participants_count,
  a.latitude,
  a.longitude,
  (
    SELECT group_concat(photo_url, '\n')
    FROM (
      SELECT ap.photo_url
      FROM activity_participants ap
      WHERE ap.activity_id = a.activity_id
        AND ap.is_deleted = 0
        AND ap.photo_url IS NOT NULL
        AND ap.photo_url != ''
      ORDER BY ap.joined_at ASC
      LIMIT 8
    )
  ) AS avatar_urls,
  (
    SELECT json_group_array(
      json_object(
        'user_id', user_id,
        'photo_url', photo_url,
        'name', name,
        'is_verified', is_verified,
        'is_friend', is_friend
      )
    )
    FROM (
      SELECT
        ap.photo_url AS photo_url,
        ap.user_id AS user_id,
        COALESCE(NULLIF(TRIM(ap.name), ''), NULLIF(TRIM(u.name), ''), 'deelnemer') AS name,
        COALESCE(u.is_verified, 0) AS is_verified,
        CASE WHEN f.friendship_id IS NOT NULL THEN 1 ELSE 0 END AS is_friend
      FROM activity_participants ap
      LEFT JOIN users u ON u.user_id = ap.user_id
      LEFT JOIN friends f ON (
        (f.friendship_id = ? || ':' || ap.user_id OR f.friendship_id = ap.user_id || ':' || ?)
        AND f.status = 'accepted'
        AND (f.is_deleted = 0 OR f.is_deleted IS NULL)
      )
      WHERE ap.activity_id = a.activity_id
        AND ap.is_deleted = 0
        AND ap.photo_url IS NOT NULL
        AND ap.photo_url != ''
      ORDER BY ap.joined_at ASC
      LIMIT 30
    )
  ) AS participants_preview_json,
  COALESCE(s.waitlist_enabled, 1) AS waitlist_enabled,
  CASE WHEN datetime(a.scheduled_at) <= datetime('now') THEN 1 ELSE 0 END AS is_past
FROM activities a
LEFT JOIN activity_settings s
  ON s.activity_id = a.activity_id
WHERE a.is_deleted = 0
  AND a.is_joined = 1
  AND datetime(a.scheduled_at) <= datetime('now')
  AND (
    ? = ''
    OR lower(a.title) LIKE ?
    OR lower(COALESCE(a.city, '')) LIKE ?
    OR lower(COALESCE(json_extract(a.location, '$.venue_name'), '')) LIKE ?
  )
  AND (
    ? IS NULL
    OR (
      a.latitude BETWEEN ? AND ?
      AND a.longitude BETWEEN ? AND ?
    )
  )
ORDER BY datetime(a.scheduled_at) DESC
LIMIT ?
"#;

pub async fn list_history(
    pool: &SqlitePool,
    auth_user_id: &str,
    q_like: &str,
    bbox: Option<(f64, f64, f64, f64)>,
    limit: i64,
) -> sqlx::Result<Vec<ActivityFeedRow>> {
    let (min_lat, max_lat, min_lon, max_lon) = bbox
        .map(|v| (Some(v.0), Some(v.1), Some(v.2), Some(v.3)))
        .unwrap_or((None, None, None, None));

    sqlx::query_as::<_, ActivityFeedRow>(SQL_LIST_HISTORY)
        .bind(auth_user_id)
        .bind(auth_user_id)
        .bind(q_like)
        .bind(q_like)
        .bind(q_like)
        .bind(q_like)
        .bind(min_lat)
        .bind(min_lat)
        .bind(max_lat)
        .bind(min_lon)
        .bind(max_lon)
        .bind(limit)
        .fetch_all(pool)
        .await
}
