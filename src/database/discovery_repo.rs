use sqlx::{sqlite::SqliteArguments, Arguments, SqlitePool};

use crate::models::{DiscoveryUserRow, UserPreferencesRow, UserProfilesRow};

pub const SQL_DISCOVERY_BASE: &str = r#"
SELECT
    u.user_id, u.name, u.city, u.main_photo_url, u.is_verified,
    u.age, u.gender, u.latitude, u.longitude,
    CASE WHEN f.friendship_id IS NOT NULL THEN 1 ELSE 0 END as is_friend
FROM users u
LEFT JOIN friends f ON (
    (f.friendship_id = ? || ':' || u.user_id OR f.friendship_id = u.user_id || ':' || ?)
    AND f.status = 'accepted'
    AND (f.is_deleted = 0 OR f.is_deleted IS NULL)
)
WHERE (u.is_deleted = 0 OR u.is_deleted IS NULL)
    AND u.main_photo_url IS NOT NULL
    AND u.main_photo_url != ''
"#;

pub const SQL_LOAD_USER_PROFILE_CONTEXT: &str = r#"
SELECT
    search_radius,
    filter_min_age,
    filter_max_age,
    filter_gender,
    latitude,
    longitude
FROM user_profiles
WHERE user_id = ?1
"#;

pub const SQL_LOAD_USER_PREFERENCES_CONTEXT: &str = r#"
SELECT
    search_radius,
    filter_min_age,
    filter_max_age,
    filter_gender,
    search_latitude,
    search_longitude
FROM user_preferences
WHERE user_id = ?1
"#;

pub async fn load_user_profile_context(
    pool: &SqlitePool,
    user_id: &str,
) -> sqlx::Result<Option<UserProfilesRow>> {
    sqlx::query_as::<_, UserProfilesRow>(SQL_LOAD_USER_PROFILE_CONTEXT)
        .bind(user_id)
        .fetch_optional(pool)
        .await
}

pub async fn load_user_preferences_context(
    pool: &SqlitePool,
    user_id: &str,
) -> sqlx::Result<Option<UserPreferencesRow>> {
    sqlx::query_as::<_, UserPreferencesRow>(SQL_LOAD_USER_PREFERENCES_CONTEXT)
        .bind(user_id)
        .fetch_optional(pool)
        .await
}

pub async fn load_discovery_candidates(
    pool: &SqlitePool,
    auth_user_id: &str,
    bbox: Option<(f64, f64, f64, f64)>,
) -> sqlx::Result<Vec<DiscoveryUserRow>> {
    let mut sql = String::from(SQL_DISCOVERY_BASE);
    let mut args = SqliteArguments::default();
    args.add(auth_user_id); // JOIN param 1
    args.add(auth_user_id); // JOIN param 2

    sql.push_str(" AND u.user_id != ?");
    args.add(auth_user_id);

    if let Some((min_lat, max_lat, min_lon, max_lon)) = bbox {
        sql.push_str(" AND latitude BETWEEN ? AND ? AND longitude BETWEEN ? AND ?");
        args.add(min_lat);
        args.add(max_lat);
        args.add(min_lon);
        args.add(max_lon);
    }

    sql.push_str(" LIMIT 500");

    sqlx::query_as_with::<_, DiscoveryUserRow, _>(&sql, args)
        .fetch_all(pool)
        .await
}
