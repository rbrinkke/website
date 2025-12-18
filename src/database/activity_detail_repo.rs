use sqlx::SqlitePool;

use crate::models::{ActivitiesRow, ActivityParticipantsRow};

const SQL_LOAD_ACTIVITY_BY_ID: &str = r#"
SELECT
  activity_id,
  title,
  description,
  activity_type,
  privacy_level,
  status,
  scheduled_at,
  duration_minutes,
  joinable_at_free,
  max_participants,
  current_participants_count,
  waitlist_count,
  city,
  distance_km,
  latitude,
  longitude,
  location,
  organizer,
  tags,
  category,
  my_role,
  my_participation_status,
  my_attendance_status,
  am_on_waitlist,
  my_waitlist_position,
  main_photo_asset_id,
  is_joined,
  can_manage_activity,
  can_manage_attendance
FROM activities
WHERE activity_id = ?
  AND is_deleted = 0
LIMIT 1
"#;

pub async fn load_activity_by_id(
    pool: &SqlitePool,
    activity_id: &str,
) -> sqlx::Result<Option<ActivitiesRow>> {
    sqlx::query_as::<_, ActivitiesRow>(SQL_LOAD_ACTIVITY_BY_ID)
        .bind(activity_id)
        .fetch_optional(pool)
        .await
}

const SQL_LIST_ACTIVITY_PARTICIPANTS: &str = r#"
SELECT
  activity_id,
  user_id,
  name,
  photo_url,
  role,
  participation_status,
  attendance_status,
  joined_at,
  updated_at,
  is_deleted
FROM activity_participants
WHERE activity_id = ?
  AND is_deleted = 0
ORDER BY
  CASE COALESCE(participation_status, '')
    WHEN 'registered' THEN 0
    WHEN 'waitlisted' THEN 1
    ELSE 2
  END,
  datetime(COALESCE(joined_at, updated_at)) ASC
"#;

pub async fn list_activity_participants(
    pool: &SqlitePool,
    activity_id: &str,
) -> sqlx::Result<Vec<ActivityParticipantsRow>> {
    sqlx::query_as::<_, ActivityParticipantsRow>(SQL_LIST_ACTIVITY_PARTICIPANTS)
        .bind(activity_id)
        .fetch_all(pool)
        .await
}
