use sqlx::SqlitePool;

use crate::database::user_summary_repo;

pub struct UserSummaryView {
    pub user_id: String,
    pub name: String,
    pub profile_description: Option<String>,
    pub age: Option<i64>,
    pub gender_label: Option<String>,
    pub location_label: String,
    pub is_verified: bool,
    pub friendship_status: String, // none|pending_outgoing|pending_incoming|accepted|blocked
    pub interests: Vec<String>,
    pub subscription_level: String,
    pub last_seen_label: Option<String>,
    pub chat_conversation_id: Option<String>,
}

pub async fn load_user_summary_view(
    pool: &SqlitePool,
    auth_user_id: &str,
    user_id: &str,
) -> sqlx::Result<Option<UserSummaryView>> {
    let Some(row) = user_summary_repo::load_user_summary(pool, auth_user_id, user_id).await? else {
        return Ok(None);
    };

    let gender_label = match row.gender.as_deref().unwrap_or("").to_lowercase().as_str() {
        "male" => Some("Man".to_string()),
        "female" => Some("Vrouw".to_string()),
        "non_binary" | "non-binary" | "nonbinary" => Some("Non-binary".to_string()),
        _ => None,
    };

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

    let profile_description = row
        .profile_description
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());

    Ok(Some(UserSummaryView {
        user_id: row.user_id,
        name: row.name.unwrap_or_else(|| "Onbekend".to_string()),
        profile_description,
        age: row.age,
        gender_label,
        location_label,
        is_verified: row.is_verified.unwrap_or(0) == 1,
        friendship_status: compute_friendship_status(
            row.friendship_status.as_deref(),
            row.initiated_by_me.unwrap_or(0) == 1,
        ),
        interests,
        subscription_level,
        last_seen_label,
        chat_conversation_id: row.chat_conversation_id,
    }))
}

fn compute_friendship_status(status: Option<&str>, initiated_by_me: bool) -> String {
    match status.unwrap_or("").trim() {
        "accepted" => "accepted".to_string(),
        "blocked" => "blocked".to_string(),
        "pending" => {
            if initiated_by_me {
                "pending_outgoing".to_string()
            } else {
                "pending_incoming".to_string()
            }
        }
        _ => "none".to_string(),
    }
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
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }
    let mut s = raw.to_string();
    if let Some(t_pos) = s.find('T') {
        s.replace_range(t_pos..=t_pos, " ");
    }
    Some(s.chars().take(16).collect())
}
