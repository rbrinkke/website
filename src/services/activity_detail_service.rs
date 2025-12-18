use serde::Deserialize;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::database::{
    activity_detail_repo, activity_signup_commands_repo, activity_waitlist_commands_repo,
};
use crate::models::{ActivitiesRow, ActivityParticipantsRow};

#[derive(Debug, Deserialize, Default)]
pub struct ActivityDetailQuery {
    pub notice: Option<String>,
}

#[derive(Clone)]
pub struct ActivityPersonView {
    pub user_id: String,
    pub name: String,
    pub photo_image_id: Option<String>,
    pub participation_status: String,
    pub role: Option<String>,
}

pub struct ActivityDetailView {
    pub activity_id: String,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub activity_type: String,
    pub privacy_level: String,
    pub scheduled_date_label: String,
    pub scheduled_time_label: String,
    pub duration_minutes: Option<i64>,
    pub max_participants: i64,
    pub current_participants_count: i64,
    pub waitlist_count: i64,
    pub is_full: bool,
    pub capacity_pct: i64,
    pub location_label: String,
    pub address_label: Option<String>,
    pub main_photo_asset_id: Option<String>,
    pub tags: Vec<String>,
    pub category_name: Option<String>,
    pub organizer_name: Option<String>,
    pub organizer_photo_image_id: Option<String>,
    pub is_joined: bool,
    pub can_manage_activity: bool,
    pub can_manage_attendance: bool,
    pub my_role: Option<String>,
    pub my_participation_status: Option<String>,
    pub am_on_waitlist: bool,
    pub my_waitlist_position: Option<i64>,
    pub notice: Option<String>,
    pub participants_registered: Vec<ActivityPersonView>,
    pub participants_waitlisted: Vec<ActivityPersonView>,
}

pub async fn load_activity_detail_view(
    pool: &SqlitePool,
    activity_id: &str,
    query: &ActivityDetailQuery,
) -> sqlx::Result<Option<ActivityDetailView>> {
    let Some(row) = activity_detail_repo::load_activity_by_id(pool, activity_id).await? else {
        return Ok(None);
    };
    let participants = activity_detail_repo::list_activity_participants(pool, activity_id).await?;
    Ok(Some(build_view(row, participants, query.notice.clone())))
}

fn build_view(
    row: ActivitiesRow,
    participants: Vec<ActivityParticipantsRow>,
    notice: Option<String>,
) -> ActivityDetailView {
    let (scheduled_date_label, scheduled_time_label) = format_scheduled_labels(&row.scheduled_at);
    let (location_label, address_label) =
        format_location_labels(row.city.as_deref(), row.location.as_deref());
    let tags = parse_string_array_json(row.tags.as_deref());
    let category_name = parse_category_name(row.category.as_deref());
    let (organizer_name, organizer_photo_image_id) = parse_organizer(row.organizer.as_str());

    let mut participants_registered = Vec::new();
    let mut participants_waitlisted = Vec::new();
    for p in participants {
        let status = p
            .participation_status
            .clone()
            .unwrap_or_else(|| "unknown".to_string());
        let name = p.name.clone().unwrap_or_else(|| "Onbekend".to_string());
        let photo_image_id = p.photo_url.as_deref().and_then(extract_image_id);

        let view = ActivityPersonView {
            user_id: p.user_id,
            name,
            photo_image_id,
            participation_status: status.clone(),
            role: p.role,
        };

        if status == "waitlisted" {
            participants_waitlisted.push(view);
        } else {
            participants_registered.push(view);
        }
    }

    ActivityDetailView {
        activity_id: row.activity_id,
        title: row.title,
        description: row.description,
        status: row.status,
        activity_type: row.activity_type,
        privacy_level: row.privacy_level,
        scheduled_date_label,
        scheduled_time_label,
        duration_minutes: row.duration_minutes,
        max_participants: row.max_participants,
        current_participants_count: row.current_participants_count,
        waitlist_count: row.waitlist_count,
        is_full: row.current_participants_count >= row.max_participants,
        capacity_pct: compute_capacity_pct(row.current_participants_count, row.max_participants),
        location_label,
        address_label,
        main_photo_asset_id: row.main_photo_asset_id,
        tags,
        category_name,
        organizer_name,
        organizer_photo_image_id,
        is_joined: row.is_joined == 1,
        can_manage_activity: row.can_manage_activity == 1,
        can_manage_attendance: row.can_manage_attendance == 1,
        my_role: row.my_role,
        my_participation_status: row.my_participation_status,
        am_on_waitlist: row.am_on_waitlist == 1,
        my_waitlist_position: row.my_waitlist_position,
        notice,
        participants_registered,
        participants_waitlisted,
    }
}

pub async fn create_signup_command(
    pool: &SqlitePool,
    actor_user_id: &str,
    activity_id: &str,
    subject_user_id: &str,
    action: &str,
) -> sqlx::Result<()> {
    let id = Uuid::new_v4().to_string();
    activity_signup_commands_repo::insert_signup_command(
        pool,
        activity_signup_commands_repo::NewActivitySignupCommand {
            id: &id,
            actor_user_id,
            activity_id,
            subject_user_id,
            action,
            note: Some("website"),
        },
    )
    .await?;
    Ok(())
}

pub async fn create_waitlist_command(
    pool: &SqlitePool,
    actor_user_id: &str,
    activity_id: &str,
    subject_user_id: &str,
    action: &str,
    priority: Option<i64>,
) -> sqlx::Result<()> {
    let id = Uuid::new_v4().to_string();
    activity_waitlist_commands_repo::insert_waitlist_command(
        pool,
        activity_waitlist_commands_repo::NewActivityWaitlistCommand {
            id: &id,
            actor_user_id,
            activity_id,
            subject_user_id,
            action,
            priority,
            note: Some("website"),
        },
    )
    .await?;
    Ok(())
}

fn format_scheduled_labels(scheduled_at: &str) -> (String, String) {
    let date = scheduled_at.get(0..10).unwrap_or(scheduled_at);
    let time = scheduled_at.get(11..16).unwrap_or("");
    (format_date_nl_short(date), time.to_string())
}

fn format_date_nl_short(date: &str) -> String {
    let (y, m, d) = match parse_ymd(date) {
        Some(v) => v,
        None => return date.to_string(),
    };

    let wd = weekday_sun0(y, m, d);
    let wd_name = match wd {
        0 => "zo",
        1 => "ma",
        2 => "di",
        3 => "wo",
        4 => "do",
        5 => "vr",
        6 => "za",
        _ => "",
    };

    let month = match m {
        1 => "jan",
        2 => "feb",
        3 => "mrt",
        4 => "apr",
        5 => "mei",
        6 => "jun",
        7 => "jul",
        8 => "aug",
        9 => "sep",
        10 => "okt",
        11 => "nov",
        12 => "dec",
        _ => "",
    };

    format!("{} {} {}", wd_name, d, month)
}

fn parse_ymd(date: &str) -> Option<(i32, i32, i32)> {
    let mut parts = date.split('-');
    let y: i32 = parts.next()?.parse().ok()?;
    let m: i32 = parts.next()?.parse().ok()?;
    let d: i32 = parts.next()?.parse().ok()?;
    Some((y, m, d))
}

fn weekday_sun0(y: i32, m: i32, d: i32) -> i32 {
    let t = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let mut year = y;
    if m < 3 {
        year -= 1;
    }
    (year + year / 4 - year / 100 + year / 400 + t[(m - 1) as usize] + d) % 7
}

fn format_location_labels(
    city: Option<&str>,
    location_json: Option<&str>,
) -> (String, Option<String>) {
    #[derive(Deserialize, Default)]
    struct Loc {
        venue_name: Option<String>,
        address_line1: Option<String>,
        postal_code: Option<String>,
        city: Option<String>,
        country: Option<String>,
    }

    let parsed: Loc = location_json
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();

    let label = parsed
        .venue_name
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .or_else(|| city.map(|s| s.to_string()))
        .or_else(|| parsed.city.clone())
        .unwrap_or_else(|| "Locatie onbekend".to_string());

    let mut addr_parts = Vec::new();
    if let Some(v) = parsed
        .address_line1
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        addr_parts.push(v.to_string());
    }
    if let Some(v) = parsed
        .postal_code
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        addr_parts.push(v.to_string());
    }
    if let Some(v) = parsed
        .city
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        addr_parts.push(v.to_string());
    }
    if let Some(v) = parsed
        .country
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        addr_parts.push(v.to_string());
    }

    let address_label = if addr_parts.is_empty() {
        None
    } else {
        Some(addr_parts.join(" • "))
    };

    (label, address_label)
}

fn parse_string_array_json(json: Option<&str>) -> Vec<String> {
    let Some(raw) = json else {
        return Vec::new();
    };
    serde_json::from_str::<Vec<String>>(raw).unwrap_or_default()
}

fn parse_category_name(json: Option<&str>) -> Option<String> {
    #[derive(Deserialize)]
    struct Cat {
        name: Option<String>,
    }
    json.and_then(|raw| serde_json::from_str::<Cat>(raw).ok())
        .and_then(|c| c.name)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn parse_organizer(json: &str) -> (Option<String>, Option<String>) {
    #[derive(Deserialize, Default)]
    struct Org {
        name: Option<String>,
        photo_url: Option<String>,
    }
    let parsed: Org = serde_json::from_str(json).unwrap_or_default();
    let name = parsed
        .name
        .as_ref()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let photo_id = parsed.photo_url.as_deref().and_then(extract_image_id);
    (name, photo_id)
}

fn extract_image_id(value: &str) -> Option<String> {
    let v = value.trim();
    if v.is_empty() {
        return None;
    }
    if let Some(pos) = v.find("/api/v1/images/") {
        let rest = &v[pos + "/api/v1/images/".len()..];
        let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
        let candidate = rest.get(0..end)?.trim();
        if candidate.len() >= 8 {
            return Some(candidate.to_string());
        }
    }
    if v.len() >= 8 && !v.contains('/') && !v.contains('?') {
        return Some(v.to_string());
    }
    None
}

fn compute_capacity_pct(current: i64, max: i64) -> i64 {
    if max <= 0 {
        return 0;
    }
    let pct = (current.saturating_mul(100)) / max;
    pct.clamp(0, 100)
}
