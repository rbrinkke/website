use sqlx::SqlitePool;

use crate::database::user_repo;

pub struct UserProfileView {
    pub name: String,
    pub profile_description: String,
    pub age: Option<i64>,
    pub gender_label: String,
    pub location_label: String,
    pub main_photo_id: Option<String>,
    pub extra_photo_ids: Vec<String>,
    pub is_verified: bool,
    pub interests: Vec<String>,
    pub subscription_level: String,
    pub activities_created_count: i64,
    pub activities_attended_count: i64,
    pub last_seen_label: Option<String>,
}

pub async fn load_user_profile_view(
    pool: &SqlitePool,
    user_id: &str,
) -> sqlx::Result<Option<UserProfileView>> {
    let Some(row) = user_repo::load_user_profile(pool, user_id).await? else {
        return Ok(None);
    };

    let extra_photo_ids = row
        .profile_photos_extra
        .as_deref()
        .unwrap_or("[]")
        .trim()
        .to_string();
    let extra_photo_ids: Vec<String> = serde_json::from_str::<Vec<String>>(&extra_photo_ids)
        .unwrap_or_default()
        .into_iter()
        .filter(|s| !s.trim().is_empty())
        .collect();

    let gender_label = match row.gender.as_deref().unwrap_or("").to_lowercase().as_str() {
        "male" => "Man",
        "female" => "Vrouw",
        "non_binary" | "non-binary" | "nonbinary" => "Non-binary",
        _ => "",
    }
    .to_string();

    let interests = parse_interest_names(row.interests.as_deref().unwrap_or("[]"));
    let subscription_level = row
        .subscription_level
        .unwrap_or_else(|| "free".to_string())
        .to_lowercase();

    let location_label = build_location_label(
        row.city.as_deref().unwrap_or("").trim(),
        row.country.as_deref().unwrap_or("").trim(),
    );
    let last_seen_label = row.last_seen_at.as_deref().and_then(format_last_seen);

    Ok(Some(UserProfileView {
        name: row.name.unwrap_or_default(),
        profile_description: row.profile_description.unwrap_or_default(),
        age: row.age,
        gender_label,
        location_label,
        main_photo_id: row.main_photo_url,
        extra_photo_ids,
        is_verified: row.is_verified.unwrap_or(0) == 1,
        interests,
        subscription_level,
        activities_created_count: row.activities_created_count.unwrap_or(0),
        activities_attended_count: row.activities_attended_count.unwrap_or(0),
        last_seen_label,
    }))
}

fn parse_interest_names(raw: &str) -> Vec<String> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(raw) else {
        return vec![];
    };
    let Some(array) = value.as_array() else {
        return vec![];
    };

    array
        .iter()
        .filter_map(|v| v.get("name").and_then(|n| n.as_str()).map(|s| s.trim()))
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

fn build_location_label(city: &str, country: &str) -> String {
    match (city.is_empty(), country.is_empty()) {
        (true, true) => String::new(),
        (false, true) => city.to_string(),
        (true, false) => country.to_string(),
        (false, false) => format!("{} · {}", city, country),
    }
}

fn format_last_seen(raw: &str) -> Option<String> {
    // Expected examples: "2025-12-12T08:06:12.920925"
    // Keep it dependency-free (no chrono) for easy Flutter parity.
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }
    let mut s = raw.to_string();
    if let Some(t_pos) = s.find('T') {
        s.replace_range(t_pos..=t_pos, " ");
    }
    // "YYYY-MM-DD HH:MM"
    Some(s.chars().take(16).collect())
}
