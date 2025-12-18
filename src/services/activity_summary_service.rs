use serde::Deserialize;
use sqlx::SqlitePool;

use crate::database::activity_summary_repo;

pub struct ActivitySummaryView {
    pub activity_id: String,
    pub description: Option<String>,
    pub scheduled_date_label: String,
    pub scheduled_time_label: String,
    pub duration_label: Option<String>,
    pub location_label: String,
    pub address_label: Option<String>,
    pub tags: Vec<String>,
    pub category_name: Option<String>,
    pub organizer_name: Option<String>,
    pub organizer_photo_image_id: Option<String>,
    pub max_participants: i64,
    pub current_participants_count: i64,
    pub waitlist_count: i64,
    pub is_full: bool,
    pub status: String,
    pub is_joined: bool,
    pub am_on_waitlist: bool,
    pub waitlist_enabled: bool,
    pub tab: Option<String>,
    pub return_to: Option<String>,
}

pub async fn load_activity_summary_view(
    pool: &SqlitePool,
    activity_id: &str,
    tab: Option<String>,
    return_to: Option<String>,
) -> sqlx::Result<Option<ActivitySummaryView>> {
    let Some(row) = activity_summary_repo::load_activity_summary(pool, activity_id).await? else {
        return Ok(None);
    };
    Ok(Some(build_view(row, tab, return_to)))
}

fn build_view(
    row: activity_summary_repo::ActivitySummaryRow,
    tab: Option<String>,
    return_to: Option<String>,
) -> ActivitySummaryView {
    let (scheduled_date_label, scheduled_time_label) = format_scheduled_labels(&row.scheduled_at);
    let (location_label, address_label) =
        format_location_labels(row.city.as_deref(), row.location.as_deref());
    let tags = parse_string_array_json(row.tags.as_deref());
    let category_name = parse_category_name(row.category.as_deref());
    let (organizer_name, organizer_photo_image_id) = parse_organizer(row.organizer.as_str());

    let duration_label = row.duration_minutes.map(|m| format!("{} min", m));
    let description = row
        .description
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());

    ActivitySummaryView {
        activity_id: row.activity_id,
        description,
        scheduled_date_label,
        scheduled_time_label,
        duration_label,
        location_label,
        address_label,
        tags,
        category_name,
        organizer_name,
        organizer_photo_image_id,
        max_participants: row.max_participants,
        current_participants_count: row.current_participants_count,
        waitlist_count: row.waitlist_count,
        is_full: row.current_participants_count >= row.max_participants,
        status: row.status,
        is_joined: row.is_joined == 1,
        am_on_waitlist: row.am_on_waitlist == 1,
        waitlist_enabled: row.waitlist_enabled == 1,
        tab,
        return_to,
    }
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

    format!("{wd_name} {d} {month}")
}

fn parse_ymd(date: &str) -> Option<(i32, i32, i32)> {
    let y = date.get(0..4)?.parse::<i32>().ok()?;
    let m = date.get(5..7)?.parse::<i32>().ok()?;
    let d = date.get(8..10)?.parse::<i32>().ok()?;
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
