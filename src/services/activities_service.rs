use serde::Deserialize;
use sqlx::SqlitePool;
use std::collections::HashSet;

use crate::database::{activities_repo, discovery_repo, interests_repo, promotion_units_repo};
use crate::models::PromotionUnitRow;

#[derive(Debug, Deserialize, Default)]
pub struct ActivitiesQuery {
    pub tab: Option<String>, // upcoming|discover|history
    pub q: Option<String>,
    pub radius_km: Option<i64>,
    pub lat: Option<f64>,
    pub lon: Option<f64>,
    pub loc_label: Option<String>,
    pub interests: Option<Vec<String>>,
    pub hide_full: Option<bool>,
    pub notice: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivitiesTab {
    Upcoming,
    Discover,
    History,
}

impl ActivitiesTab {
    pub fn as_str(self) -> &'static str {
        match self {
            ActivitiesTab::Upcoming => "upcoming",
            ActivitiesTab::Discover => "discover",
            ActivitiesTab::History => "history",
        }
    }
}

fn parse_tab(input: Option<&str>) -> ActivitiesTab {
    match input.unwrap_or("discover") {
        "upcoming" => ActivitiesTab::Upcoming,
        "history" => ActivitiesTab::History,
        _ => ActivitiesTab::Discover,
    }
}

#[derive(Clone, Default)]
pub struct AppliedActivityFilters {
    pub tab: String,
    pub search_query: String,
    pub radius_km: i64,
    pub lat: Option<f64>,
    pub lon: Option<f64>,
    pub coord_label: Option<String>,
    pub location_label: Option<String>,
    pub selected_interests: Vec<String>,
    pub hide_full: bool,
    pub notice: Option<String>,
}

#[derive(Clone)]
pub struct InterestOptionView {
    pub name: String,
    pub emoji: Option<String>,
    pub selected: bool,
}

pub struct ActivityCardView {
    pub activity_id: String,
    pub title: String,
    pub scheduled_at: String,
    pub date_label: String,
    pub time_label: String,
    pub location_label: String,
    pub organizer_name: Option<String>,
    pub organizer_photo_image_id: Option<String>,
    pub main_photo_asset_id: Option<String>,
    pub participants_preview: Vec<ActivityParticipantPreview>,
    pub participants_page_size: usize,
    pub max_participants: i64,
    pub participants_count: i64,
    pub is_full: bool,
    pub waitlist_enabled: bool,
    pub status: String,
    pub is_joined: bool,
    pub is_past: bool,
    pub distance_km: Option<f64>,
}

#[derive(Clone)]
pub struct ActivityParticipantPreview {
    pub image_id: Option<String>,
    pub user_id: Option<String>,
    pub name: String,
    pub is_verified: bool,
    pub is_friend: bool,
    pub is_organizer: bool,
    pub is_cta: bool,
    pub is_filler: bool,
    pub cta_label: Option<String>,
    pub cta_icon: Option<String>,
    pub cta_action_kind: Option<String>,
    pub cta_background_color: Option<String>,
    pub cta_background_gradient: Option<String>,
    pub filler_background_color: Option<String>,
}

pub struct ActivitiesPageData {
    pub tab: ActivitiesTab,
    pub activities: Vec<ActivityCardView>,
    pub filters: AppliedActivityFilters,
    pub interest_options: Vec<InterestOptionView>,
}

#[derive(Default)]
struct UserContext {
    lat: Option<f64>,
    lon: Option<f64>,
    default_radius: i64,
}

pub async fn build_activities_page(
    pool: &SqlitePool,
    auth_user_id: &str,
    query: &ActivitiesQuery,
) -> sqlx::Result<ActivitiesPageData> {
    let promo_units = promotion_units_repo::list_active_for_placement(
        pool,
        "activities_participants_cta",
        "default",
        "nl",
    )
    .await
    .unwrap_or_default();

    let info_units = promotion_units_repo::list_active_for_placement(
        pool,
        "activities_participants_info",
        "default",
        "nl",
    )
    .await
    .unwrap_or_default();

    let user_ctx = load_user_context(pool, auth_user_id)
        .await
        .unwrap_or_default();

    let tab = parse_tab(query.tab.as_deref());
    let effective = merge_filters(query, &user_ctx, tab);

    let bbox = effective
        .lat
        .zip(effective.lon)
        .map(|(lat, lon)| bounding_box(lat, lon, effective.radius_km as f64));

    let q_like = if effective.search_query.trim().is_empty() {
        String::new()
    } else {
        format!("%{}%", effective.search_query.trim().to_lowercase())
    };

    let limit = 200;
    let rows = match tab {
        ActivitiesTab::Upcoming => {
            activities_repo::list_upcoming(pool, auth_user_id, &q_like, bbox, limit).await?
        }
        ActivitiesTab::Discover => {
            activities_repo::list_discover(pool, auth_user_id, &q_like, bbox, limit).await?
        }
        ActivitiesTab::History => {
            activities_repo::list_history(pool, auth_user_id, &q_like, bbox, limit).await?
        }
    };

    let interest_rows = interests_repo::list_active(pool, 24)
        .await
        .unwrap_or_default();
    let interest_options = interest_rows
        .into_iter()
        .map(|i| {
            let selected = effective
                .selected_interests
                .iter()
                .any(|s| s.eq_ignore_ascii_case(i.name.trim()));
            InterestOptionView {
                name: i.name.trim().to_string(),
                emoji: i.emoji.and_then(|e| {
                    let t = e.trim().to_string();
                    if t.is_empty() {
                        None
                    } else {
                        Some(t)
                    }
                }),
                selected,
            }
        })
        .collect::<Vec<_>>();

    let selected_interest_set: HashSet<String> = effective
        .selected_interests
        .iter()
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .collect();

    let filler_emoji = match tab {
        ActivitiesTab::Upcoming => Some("🎉"),
        ActivitiesTab::History => Some("✅"),
        ActivitiesTab::Discover => None,
    };

    let mut cards = Vec::new();
    for row in rows {
        if effective.hide_full
            && row.current_participants_count >= row.max_participants
            && row.is_joined == 0
        {
            continue;
        }

        if !selected_interest_set.is_empty() {
            let tags = parse_string_array_json(row.tags.as_deref());
            let matches = tags.into_iter().any(|t| {
                let t = t.trim().to_lowercase();
                !t.is_empty() && selected_interest_set.contains(&t)
            });
            if !matches {
                continue;
            }
        }

        let mut distance_km = None;
        if let (Some(lat0), Some(lon0), Some(lat1), Some(lon1)) =
            (effective.lat, effective.lon, row.latitude, row.longitude)
        {
            let dist = haversine_km(lat0, lon0, lat1, lon1);
            if dist > effective.radius_km as f64 {
                continue;
            }
            distance_km = Some(dist);
        }

        let (date_label, time_label) = format_scheduled_labels(&row.scheduled_at);
        let location_label = row
            .venue_name
            .clone()
            .filter(|s| !s.trim().is_empty())
            .or_else(|| row.city.clone().filter(|s| !s.trim().is_empty()))
            .unwrap_or_else(|| "Locatie onbekend".to_string());

        let participants_preview =
            parse_participants_preview(row.participants_preview_json.as_deref());

        let organizer_name = row
            .organizer_name
            .clone()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let organizer_photo_image_id = row
            .organizer_photo_asset_id
            .clone()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        let participants_preview = prepend_organizer_participant(
            participants_preview,
            organizer_name.clone(),
            row.organizer_user_id.clone(),
            organizer_photo_image_id.clone(),
        );

        let participants_preview = reorder_friends_after_organizer(participants_preview);

        let waitlist_enabled = row.waitlist_enabled == 1;
        let desired_action_kind = if row.current_participants_count >= row.max_participants
            && row.is_past == 0
            && row.status != "cancelled"
            && row.is_joined == 0
            && waitlist_enabled
        {
            "waitlist"
        } else if row.is_past == 0 && row.status != "cancelled" && row.is_joined == 0 {
            "join"
        } else {
            "view"
        };

        let (participants_preview, participants_page_size) = build_first_page_with_ctas(
            participants_preview,
            &promo_units,
            &info_units,
            desired_action_kind,
            &format!("{}:{}", row.activity_id, desired_action_kind),
            filler_emoji,
        );

        cards.push(ActivityCardView {
            activity_id: row.activity_id,
            title: row.title,
            scheduled_at: row.scheduled_at.clone(),
            date_label,
            time_label,
            location_label,
            organizer_name,
            organizer_photo_image_id,
            main_photo_asset_id: row.main_photo_asset_id,
            participants_preview,
            participants_page_size,
            max_participants: row.max_participants,
            participants_count: row.current_participants_count,
            is_full: row.current_participants_count >= row.max_participants,
            waitlist_enabled,
            status: row.status,
            is_joined: row.is_joined == 1,
            is_past: row.is_past == 1,
            distance_km,
        });
    }

    match tab {
        ActivitiesTab::Discover => {
            if effective.lat.is_some() && effective.lon.is_some() {
                cards.sort_by(|a, b| {
                    a.distance_km
                        .unwrap_or(f64::MAX)
                        .partial_cmp(&b.distance_km.unwrap_or(f64::MAX))
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            } else {
                cards.sort_by(|a, b| a.scheduled_at.cmp(&b.scheduled_at));
            }
        }
        ActivitiesTab::Upcoming => {
            cards.sort_by(|a, b| a.scheduled_at.cmp(&b.scheduled_at));
        }
        ActivitiesTab::History => {
            cards.sort_by(|a, b| b.scheduled_at.cmp(&a.scheduled_at));
        }
    }

    Ok(ActivitiesPageData {
        tab,
        activities: cards,
        filters: effective,
        interest_options,
    })
}

#[derive(Debug, Deserialize)]
struct ParticipantPreviewJson {
    user_id: Option<String>,
    photo_url: Option<String>,
    name: Option<String>,
    is_verified: Option<i64>,
    is_friend: Option<i64>,
}

fn parse_participants_preview(raw_json: Option<&str>) -> Vec<ActivityParticipantPreview> {
    let Some(raw) = raw_json.map(str::trim).filter(|s| !s.is_empty()) else {
        return Vec::new();
    };

    let parsed: Vec<ParticipantPreviewJson> = match serde_json::from_str(raw) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };

    let mut out = Vec::new();
    for p in parsed.into_iter().take(30) {
        let image_id = p.photo_url.as_deref().and_then(|url| extract_image_id(url));
        let name = p
            .name
            .unwrap_or_else(|| "deelnemer".to_string())
            .trim()
            .to_string();
        let name = if name.is_empty() {
            "deelnemer".to_string()
        } else {
            name
        };

        out.push(ActivityParticipantPreview {
            image_id,
            user_id: p
                .user_id
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty()),
            name,
            is_verified: p.is_verified.unwrap_or(0) == 1,
            is_friend: p.is_friend.unwrap_or(0) == 1,
            is_organizer: false,
            is_cta: false,
            is_filler: false,
            cta_label: None,
            cta_icon: None,
            cta_action_kind: None,
            cta_background_color: None,
            cta_background_gradient: None,
            filler_background_color: None,
        });
    }

    out
}

fn prepend_organizer_participant(
    mut participants: Vec<ActivityParticipantPreview>,
    organizer_name: Option<String>,
    organizer_user_id: Option<String>,
    organizer_photo_image_id: Option<String>,
) -> Vec<ActivityParticipantPreview> {
    let name = organizer_name
        .unwrap_or_else(|| "Organisator".to_string())
        .trim()
        .to_string();
    let name = if name.is_empty() {
        "Organisator".to_string()
    } else {
        name
    };

    let image_id = organizer_photo_image_id
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    if image_id.is_none() && name == "Organisator" {
        return participants;
    }

    participants.insert(
        0,
        ActivityParticipantPreview {
            image_id,
            user_id: organizer_user_id
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty()),
            name,
            is_verified: false,
            is_friend: false,
            is_organizer: true,
            is_cta: false,
            is_filler: false,
            cta_label: None,
            cta_icon: None,
            cta_action_kind: None,
            cta_background_color: None,
            cta_background_gradient: None,
            filler_background_color: None,
        },
    );

    participants
}

fn reorder_friends_after_organizer(
    participants: Vec<ActivityParticipantPreview>,
) -> Vec<ActivityParticipantPreview> {
    if participants.len() <= 1 {
        return participants;
    }

    let mut out = Vec::with_capacity(participants.len());
    let (head, tail) = if participants[0].is_organizer {
        out.push(participants[0].clone());
        (&participants[0..1], &participants[1..])
    } else {
        (&participants[0..0], &participants[..])
    };
    let _ = head;

    let mut friends = Vec::new();
    let mut others = Vec::new();
    for p in tail {
        if p.is_friend {
            friends.push(p.clone());
        } else {
            others.push(p.clone());
        }
    }

    out.extend(friends);
    out.extend(others);
    out
}

#[derive(Debug, Deserialize)]
struct PromotionActionJson {
    kind: Option<String>,
    label: Option<String>,
    icon: Option<String>,
    #[allow(dead_code)]
    method: Option<String>,
    #[allow(dead_code)]
    href: Option<String>,
    #[allow(dead_code)]
    style: Option<String>,
}

fn extract_primary_action(unit: &PromotionUnitRow) -> Option<PromotionActionJson> {
    let raw = unit.actions_json.as_deref()?.trim();
    if raw.is_empty() {
        return None;
    }
    let parsed: Vec<PromotionActionJson> = serde_json::from_str(raw).ok()?;
    parsed
        .into_iter()
        .find(|a| a.kind.as_deref().unwrap_or("").trim() != "")
}

fn choose_weighted_index(seed: u64, weights: &[u64]) -> Option<usize> {
    let total: u64 = weights.iter().sum();
    if total == 0 {
        return None;
    }
    let mut pick = seed % total;
    for (idx, w) in weights.iter().enumerate() {
        if *w == 0 {
            continue;
        }
        if pick < *w {
            return Some(idx);
        }
        pick -= *w;
    }
    None
}

fn stable_seed_u64(input: &str) -> u64 {
    // Very small stable hash (FNV-1a 64-bit) to avoid adding RNG deps.
    let mut hash: u64 = 14695981039346656037;
    for b in input.as_bytes() {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(1099511628211);
    }
    hash
}

fn pick_cta_tile(
    units: &[PromotionUnitRow],
    desired_action_kind: &str,
    seed_key: &str,
) -> Option<ActivityParticipantPreview> {
    pick_cta_tile_excluding(units, desired_action_kind, seed_key, &HashSet::new())
}

fn cta_dedupe_key(label: &str, icon: Option<&str>) -> String {
    format!("{}|{}", label.trim(), icon.unwrap_or("").trim())
}

fn pick_cta_tile_excluding(
    units: &[PromotionUnitRow],
    desired_action_kind: &str,
    seed_key: &str,
    exclude: &HashSet<String>,
) -> Option<ActivityParticipantPreview> {
    let mut candidates: Vec<(usize, u64)> = Vec::new();
    for (i, u) in units.iter().enumerate() {
        let Some(action) = extract_primary_action(u) else {
            continue;
        };
        if action.kind.as_deref()? != desired_action_kind {
            continue;
        }

        let label = u
            .body
            .clone()
            .or_else(|| u.title.clone())
            .unwrap_or_default();
        let key = cta_dedupe_key(&label, action.icon.as_deref().or(u.emoji.as_deref()));
        if exclude.contains(&key) {
            continue;
        }

        let w = u.weight.max(0) as u64;
        candidates.push((i, if w == 0 { 1 } else { w }));
    }
    if candidates.is_empty() {
        return None;
    }

    let seed = stable_seed_u64(seed_key);
    let weights: Vec<u64> = candidates.iter().map(|(_, w)| *w).collect();
    let picked = choose_weighted_index(seed, &weights).unwrap_or(0);
    let unit = &units[candidates[picked].0];
    let action = extract_primary_action(unit)?;

    let label = unit
        .body
        .clone()
        .or_else(|| unit.title.clone())
        .unwrap_or_default();

    Some(ActivityParticipantPreview {
        image_id: None,
        user_id: None,
        name: label,
        is_verified: false,
        is_friend: false,
        is_organizer: false,
        is_cta: true,
        is_filler: false,
        cta_label: action.label.clone(),
        cta_icon: action.icon.clone().or(unit.emoji.clone()),
        cta_action_kind: action.kind.clone(),
        cta_background_color: unit.background_color.clone(),
        cta_background_gradient: unit.background_gradient.clone(),
        filler_background_color: None,
    })
}

fn is_horizontally_adjacent(a: usize, b: usize) -> bool {
    if a == b {
        return true;
    }
    let (min_i, max_i) = if a < b { (a, b) } else { (b, a) };
    if max_i != min_i + 1 {
        return false;
    }
    // Adjacent only if within the same row of 5.
    (min_i % 5) != 4
}

fn filler_tile() -> ActivityParticipantPreview {
    ActivityParticipantPreview {
        image_id: None,
        user_id: None,
        name: String::new(),
        is_verified: false,
        is_friend: false,
        is_organizer: false,
        is_cta: false,
        is_filler: true,
        cta_label: None,
        cta_icon: None,
        cta_action_kind: None,
        cta_background_color: None,
        cta_background_gradient: None,
        filler_background_color: Some("#0B1220".to_string()),
    }
}

fn choose_filler_positions(count: usize, seed_key: &str, target_size: usize) -> Vec<usize> {
    if count == 0 || target_size <= 2 {
        return Vec::new();
    }

    let pick_from_sorted = |sorted: &mut Vec<usize>, picked: &Vec<usize>| -> Option<usize> {
        if let Some((i, pos)) = sorted
            .iter()
            .enumerate()
            .find(|(_, p)| !picked.iter().any(|q| is_horizontally_adjacent(*q, **p)))
        {
            let pos = *pos;
            sorted.remove(i);
            return Some(pos);
        }
        sorted.pop()
    };

    let mut picked: Vec<usize> = Vec::new();
    let mut remaining = count;

    // Candidates exclude index 1 (right after organizer).
    let mut candidates: Vec<usize> = (2..target_size).collect();

    if target_size >= 10 {
        let mut top: Vec<usize> = candidates.iter().copied().filter(|p| *p < 5).collect();
        let mut bottom: Vec<usize> = candidates.iter().copied().filter(|p| *p >= 5).collect();

        top.sort_by_key(|p| stable_seed_u64(&format!("{seed_key}:top:{p}")));
        bottom.sort_by_key(|p| stable_seed_u64(&format!("{seed_key}:bottom:{p}")));

        let start_top = (stable_seed_u64(&format!("{seed_key}:start")) % 2) == 0;
        for i in 0..remaining {
            let pick_top = if i % 2 == 0 { start_top } else { !start_top };
            let chosen = if pick_top {
                pick_from_sorted(&mut top, &picked)
                    .or_else(|| pick_from_sorted(&mut bottom, &picked))
            } else {
                pick_from_sorted(&mut bottom, &picked)
                    .or_else(|| pick_from_sorted(&mut top, &picked))
            };
            if let Some(pos) = chosen {
                picked.push(pos);
            }
        }
    } else {
        candidates.sort_by_key(|p| stable_seed_u64(&format!("{seed_key}:{p}")));
        while remaining > 0 {
            let pos = pick_from_sorted(&mut candidates, &picked);
            if let Some(pos) = pos {
                picked.push(pos);
            } else {
                break;
            }
            remaining -= 1;
        }
    }

    picked.sort_unstable();
    picked.truncate(count.min(picked.len()));
    picked
}

fn choose_cta_positions(desired: usize, seed_key: &str, target_size: usize) -> Vec<usize> {
    // Choose positions in the first 10 tiles (0..9). Avoid index 1 (right after organizer),
    // avoid horizontal adjacency, and for 2 CTAs try to split across rows.
    if desired == 0 {
        return Vec::new();
    }

    let seed = stable_seed_u64(seed_key);
    let mut candidates: Vec<usize> = (2..target_size).collect();
    let mut picked: Vec<usize> = Vec::new();

    for attempt in 0..(desired * 6).max(10) {
        if picked.len() >= desired {
            break;
        }
        if candidates.is_empty() {
            break;
        }

        let mut filtered: Vec<usize> = Vec::new();
        for p in candidates.iter().copied() {
            if picked.iter().any(|q| is_horizontally_adjacent(*q, p)) {
                continue;
            }
            filtered.push(p);
        }
        if filtered.is_empty() {
            break;
        }

        // First pick in top row if we want 2+ and nothing picked yet.
        if desired >= 2 && picked.is_empty() {
            let top: Vec<usize> = filtered.iter().copied().filter(|p| *p < 5).collect();
            if !top.is_empty() {
                let idx = (seed.wrapping_add(attempt as u64) as usize) % top.len();
                let pos = top[idx];
                picked.push(pos);
                candidates.retain(|p| *p != pos);
                continue;
            }
        }
        // Second pick in bottom row when possible.
        if desired >= 2 && picked.len() == 1 && target_size >= 10 {
            let bottom: Vec<usize> = filtered.iter().copied().filter(|p| *p >= 5).collect();
            if !bottom.is_empty() {
                let idx = (seed.wrapping_add(attempt as u64 + 101) as usize) % bottom.len();
                let pos = bottom[idx];
                picked.push(pos);
                candidates.retain(|p| *p != pos);
                continue;
            }
        }

        let idx = (seed.wrapping_add((attempt as u64 + 1) * 31) as usize) % filtered.len();
        let pos = filtered[idx];
        picked.push(pos);
        candidates.retain(|p| *p != pos);
    }

    picked.sort_unstable();
    picked
}

fn build_first_page_with_ctas(
    people: Vec<ActivityParticipantPreview>,
    units: &[PromotionUnitRow],
    info_units: &[PromotionUnitRow],
    desired_action_kind: &str,
    seed_key: &str,
    filler_emoji: Option<&str>,
) -> (Vec<ActivityParticipantPreview>, usize) {
    if people.is_empty() {
        return (Vec::new(), 0);
    }

    let target_size: usize = if people.len() <= 5 { 5 } else { 10 };
    let is_view_mode = desired_action_kind == "view";

    let organizer = people[0].clone();
    let rest = &people[1..];

    // Reserve the last tile for the "info" action.
    let content_size = target_size.saturating_sub(1);
    let max_slots_after_organizer = content_size.saturating_sub(1);
    let people_count = rest.len().min(max_slots_after_organizer);
    let empty_slots = max_slots_after_organizer.saturating_sub(people_count);

    let positions = if is_view_mode {
        choose_filler_positions(empty_slots, seed_key, content_size)
    } else {
        choose_cta_positions(empty_slots, seed_key, content_size)
    };
    let cta_count = positions.len();
    let filler_count = empty_slots.saturating_sub(cta_count);

    let brand = ["#1E88E5", "#FF0066", "#0B1220"];
    let shift = (stable_seed_u64(seed_key) % 3) as usize;

    let mut ctas = Vec::new();
    let mut seen_keys: HashSet<String> = HashSet::new();
    for i in 0..cta_count {
        let mut cta = if is_view_mode {
            let mut f = filler_tile();
            f.name = filler_emoji.unwrap_or("👥").to_string();
            f
        } else {
            pick_cta_tile_excluding(
                units,
                desired_action_kind,
                &format!("{}:{}:{}", seed_key, desired_action_kind, i + 1),
                &seen_keys,
            )
            .unwrap_or_else(|| {
                pick_cta_tile(units, desired_action_kind, seed_key).unwrap_or_else(|| {
                    ActivityParticipantPreview {
                        is_cta: true,
                        ..filler_tile()
                    }
                })
            })
        };

        // Ensure the same label/icon doesn't repeat within a single activity.
        if !is_view_mode {
            let dedupe_key = cta_dedupe_key(&cta.name, cta.cta_icon.as_deref());
            if !dedupe_key.trim().is_empty() {
                if seen_keys.contains(&dedupe_key) {
                    for attempt in 0..6 {
                        if let Some(next) = pick_cta_tile_excluding(
                            units,
                            desired_action_kind,
                            &format!(
                                "{}:{}:{}:try{}",
                                seed_key,
                                desired_action_kind,
                                i + 1,
                                attempt
                            ),
                            &seen_keys,
                        ) {
                            cta = next;
                            break;
                        }
                    }
                }
                let dedupe_key = cta_dedupe_key(&cta.name, cta.cta_icon.as_deref());
                seen_keys.insert(dedupe_key);
            }
        }

        if is_view_mode {
            cta.filler_background_color = Some(brand[(shift + i) % 3].to_string());
        } else {
            cta.cta_background_color = Some(brand[(shift + i) % 3].to_string());
        }
        ctas.push(cta);
    }

    let mut out = Vec::with_capacity(target_size);
    out.push(organizer);

    let mut person_i = 0usize;
    let mut cta_i = 0usize;
    let mut filler_left = filler_count;

    for idx in 1..content_size {
        if positions.contains(&idx) {
            out.push(ctas.get(cta_i).cloned().unwrap_or_else(filler_tile));
            cta_i += 1;
            continue;
        }
        if person_i < people_count {
            out.push(rest[person_i].clone());
            person_i += 1;
            continue;
        }
        if filler_left > 0 {
            let mut f = filler_tile();
            f.filler_background_color = Some(brand[(shift + idx) % 3].to_string());
            f.name = filler_emoji.unwrap_or("👥").to_string();
            out.push(f);
            filler_left -= 1;
            continue;
        }
        out.push(filler_tile());
    }

    out.push(build_info_tile(info_units, seed_key));

    // Append remaining people for carousel rotation.
    for p in rest.iter().skip(people_count) {
        out.push(p.clone());
    }

    (out, target_size)
}

fn build_info_tile(units: &[PromotionUnitRow], seed_key: &str) -> ActivityParticipantPreview {
    let mut tile = pick_cta_tile(units, "info", &format!("{seed_key}:info")).unwrap_or_else(|| {
        ActivityParticipantPreview {
            image_id: None,
            user_id: None,
            name: "Meer info".to_string(),
            is_verified: false,
            is_friend: false,
            is_organizer: false,
            is_cta: true,
            is_filler: false,
            cta_label: Some("Info".to_string()),
            cta_icon: Some("info".to_string()),
            cta_action_kind: Some("info".to_string()),
            cta_background_color: None,
            cta_background_gradient: None,
            filler_background_color: None,
        }
    });

    tile.is_cta = true;
    tile.cta_action_kind = Some("info".to_string());
    tile.cta_label = Some("Info".to_string());
    tile.cta_background_color = Some("#1E88E5".to_string());
    tile
}

async fn load_user_context(pool: &SqlitePool, user_id: &str) -> sqlx::Result<UserContext> {
    let mut ctx = UserContext {
        lat: None,
        lon: None,
        default_radius: 25,
    };

    if let Some(profile) = discovery_repo::load_user_profile_context(pool, user_id).await? {
        ctx.default_radius = profile.search_radius;
        if let (Some(lat), Some(lon)) = (profile.latitude, profile.longitude) {
            ctx.lat = Some(lat);
            ctx.lon = Some(lon);
        }
    } else if let Some(prefs) = discovery_repo::load_user_preferences_context(pool, user_id).await?
    {
        ctx.default_radius = prefs.search_radius;
        if let (Some(lat), Some(lon)) = (prefs.search_latitude, prefs.search_longitude) {
            ctx.lat = Some(lat);
            ctx.lon = Some(lon);
        }
    }

    Ok(ctx)
}

fn merge_filters(
    query: &ActivitiesQuery,
    ctx: &UserContext,
    tab: ActivitiesTab,
) -> AppliedActivityFilters {
    let lat = query.lat.or(ctx.lat);
    let lon = query.lon.or(ctx.lon);

    let mut selected_interests: Vec<String> = Vec::new();
    if let Some(list) = query.interests.as_ref() {
        for raw in list {
            let t = raw.trim();
            if t.is_empty() {
                continue;
            }
            if !selected_interests.iter().any(|s| s.eq_ignore_ascii_case(t)) {
                selected_interests.push(t.to_string());
            }
            if selected_interests.len() >= 10 {
                break;
            }
        }
    }

    AppliedActivityFilters {
        tab: tab.as_str().to_string(),
        search_query: query.q.clone().unwrap_or_default(),
        radius_km: query.radius_km.unwrap_or(ctx.default_radius).clamp(1, 500),
        lat,
        lon,
        coord_label: query
            .loc_label
            .clone()
            .or_else(|| lat.zip(lon).map(|(a, o)| format!("{:.4}, {:.4}", a, o))),
        location_label: query.loc_label.clone(),
        selected_interests,
        hide_full: query.hide_full.unwrap_or(false),
        notice: query.notice.clone(),
    }
}

fn parse_string_array_json(raw: Option<&str>) -> Vec<String> {
    let Some(raw) = raw.map(str::trim).filter(|s| !s.is_empty()) else {
        return Vec::new();
    };
    serde_json::from_str::<Vec<String>>(raw).unwrap_or_default()
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

fn format_scheduled_labels(scheduled_at: &str) -> (String, String) {
    // Input is an ISO-ish string like: 2025-10-17T10:06:13.256414
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

// Returns weekday with Sunday=0..Saturday=6 (Sakamoto algorithm).
fn weekday_sun0(y: i32, m: i32, d: i32) -> i32 {
    let t = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let mut year = y;
    if m < 3 {
        year -= 1;
    }
    (year + year / 4 - year / 100 + year / 400 + t[(m - 1) as usize] + d) % 7
}

fn extract_image_id(value: &str) -> Option<String> {
    let v = value.trim();
    if v.is_empty() {
        return None;
    }

    // Full URL like: https://image.localhost/api/v1/images/<uuid>/medium
    if let Some(pos) = v.find("/api/v1/images/") {
        let rest = &v[pos + "/api/v1/images/".len()..];
        let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
        let candidate = rest.get(0..end)?.trim();
        if candidate.len() >= 8 {
            return Some(candidate.to_string());
        }
    }

    // Already an id
    if v.len() >= 8 && !v.contains('/') && !v.contains('?') {
        return Some(v.to_string());
    }

    None
}
