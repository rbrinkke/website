use serde::Deserialize;
use sqlx::SqlitePool;

use crate::database::discovery_repo;
use crate::models::DiscoveryUserRow;

#[derive(Debug, Deserialize, Default)]
pub struct DiscoveryQuery {
    pub q: Option<String>,
    pub gender: Option<String>,
    pub min_age: Option<i64>,
    pub max_age: Option<i64>,
    pub radius_km: Option<i64>,
    pub lat: Option<f64>,
    pub lon: Option<f64>,
    pub friends_only: Option<bool>,
    pub loc_label: Option<String>,
}

#[derive(Clone, Default)]
pub struct AppliedFilters {
    pub search_query: String,
    pub gender_value: String,
    pub min_age_value: String,
    pub max_age_value: String,
    pub radius_km: i64,
    pub friends_only: bool,
    pub lat: Option<f64>,
    pub lon: Option<f64>,
    pub coord_label: Option<String>,
    pub location_label: Option<String>,
}

#[derive(Default)]
struct UserContext {
    lat: Option<f64>,
    lon: Option<f64>,
    default_radius: i64,
    default_min_age: Option<i64>,
    default_max_age: Option<i64>,
    default_gender: Option<String>,
}

pub struct DiscoveryPageData {
    pub users: Vec<DiscoveryUserRow>,
    pub filters: AppliedFilters,
}

pub async fn build_discovery_page(
    pool: &SqlitePool,
    auth_user_id: &str,
    query: &DiscoveryQuery,
) -> sqlx::Result<DiscoveryPageData> {
    let user_ctx = load_user_context(pool, auth_user_id)
        .await
        .unwrap_or_default();
    let effective_filters = merge_filters(query, &user_ctx);

    let bbox = effective_filters
        .lat
        .zip(effective_filters.lon)
        .map(|(lat, lon)| bounding_box(lat, lon, effective_filters.radius_km as f64));

    let rows = discovery_repo::load_discovery_candidates(pool, auth_user_id, bbox).await?;

    let mut users = Vec::new();
    for mut user in rows {
        if let (Some(lat0), Some(lon0), Some(lat1), Some(lon1)) = (
            effective_filters.lat,
            effective_filters.lon,
            user.latitude,
            user.longitude,
        ) {
            let dist = haversine_km(lat0, lon0, lat1, lon1);
            if dist > effective_filters.radius_km as f64 {
                continue;
            }
            user.distance_km = Some(dist);
        }
        users.push(user);
    }

    if effective_filters.lat.is_some() && effective_filters.lon.is_some() {
        users.sort_by(|a, b| {
            a.distance_km
                .unwrap_or(f64::MAX)
                .partial_cmp(&b.distance_km.unwrap_or(f64::MAX))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    } else {
        users.sort_by(|a, b| {
            a.name
                .as_deref()
                .unwrap_or("")
                .to_lowercase()
                .cmp(&b.name.as_deref().unwrap_or("").to_lowercase())
        });
    }

    Ok(DiscoveryPageData {
        users,
        filters: effective_filters,
    })
}

async fn load_user_context(pool: &SqlitePool, user_id: &str) -> sqlx::Result<UserContext> {
    let mut ctx = UserContext {
        lat: None,
        lon: None,
        default_radius: 25,
        default_min_age: None,
        default_max_age: None,
        default_gender: None,
    };

    if let Some(profile) = discovery_repo::load_user_profile_context(pool, user_id).await? {
        ctx.default_radius = profile.search_radius;
        ctx.default_min_age = profile.filter_min_age;
        ctx.default_max_age = profile.filter_max_age;
        ctx.default_gender = profile.filter_gender;
        if let (Some(lat), Some(lon)) = (profile.latitude, profile.longitude) {
            ctx.lat = Some(lat);
            ctx.lon = Some(lon);
        }
    } else if let Some(prefs) = discovery_repo::load_user_preferences_context(pool, user_id).await?
    {
        ctx.default_radius = prefs.search_radius;
        ctx.default_min_age = prefs.filter_min_age;
        ctx.default_max_age = prefs.filter_max_age;
        ctx.default_gender = prefs.filter_gender;
        if let (Some(lat), Some(lon)) = (prefs.search_latitude, prefs.search_longitude) {
            ctx.lat = Some(lat);
            ctx.lon = Some(lon);
        }
    }

    Ok(ctx)
}

fn merge_filters(query: &DiscoveryQuery, ctx: &UserContext) -> AppliedFilters {
    AppliedFilters {
        search_query: query.q.clone().unwrap_or_default(),
        gender_value: query
            .gender
            .clone()
            .or_else(|| ctx.default_gender.clone())
            .unwrap_or_default(),
        min_age_value: query
            .min_age
            .or(ctx.default_min_age)
            .map(|v| v.to_string())
            .unwrap_or_default(),
        max_age_value: query
            .max_age
            .or(ctx.default_max_age)
            .map(|v| v.to_string())
            .unwrap_or_default(),
        radius_km: query.radius_km.unwrap_or(ctx.default_radius),
        friends_only: query.friends_only.unwrap_or(false),
        lat: query.lat.or(ctx.lat),
        lon: query.lon.or(ctx.lon),
        coord_label: query.loc_label.clone().or_else(|| {
            query
                .lat
                .or(ctx.lat)
                .zip(query.lon.or(ctx.lon))
                .map(|(lat, lon)| format!("{:.4}, {:.4}", lat, lon))
        }),
        location_label: query.loc_label.clone(),
    }
}

fn haversine_km(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let to_rad = |deg: f64| deg.to_radians();
    let dlat = to_rad(lat2 - lat1);
    let dlon = to_rad(lon2 - lon1);
    let a = (dlat / 2.0).sin().powi(2)
        + to_rad(lat1).cos() * to_rad(lat2).cos() * (dlon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().asin();
    6371.0 * c
}

fn bounding_box(lat: f64, lon: f64, radius_km: f64) -> (f64, f64, f64, f64) {
    let lat_change = radius_km / 111.0;
    let lat_rad = lat.to_radians();
    let lon_change = (radius_km / 111.0) / lat_rad.cos().abs();

    (
        lat - lat_change,
        lat + lat_change,
        lon - lon_change,
        lon + lon_change,
    )
}
