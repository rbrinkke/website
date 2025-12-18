use serde::Deserialize;
use sqlx::SqlitePool;
use std::collections::HashMap;
use tracing::info;
use tracing::warn;

use crate::database::activity_repo;
use crate::services::location_service;

#[derive(Debug, Default)]
pub struct ActivityGeoBackfillReport {
    pub candidates: usize,
    pub updated: usize,
    pub skipped: usize,
    pub failed: usize,
}

#[derive(Debug, Deserialize, Default)]
struct ActivityLocationJson {
    venue_name: Option<String>,
    city: Option<String>,
    postal_code: Option<String>,
    country: Option<String>,
}

pub async fn backfill_activity_geo(
    pool: &SqlitePool,
    limit: i64,
) -> sqlx::Result<ActivityGeoBackfillReport> {
    let candidates = activity_repo::list_activities_missing_geo(pool, limit).await?;
    let mut report = ActivityGeoBackfillReport {
        candidates: candidates.len(),
        ..Default::default()
    };

    let mut cache: HashMap<String, (f64, f64)> = HashMap::new();

    for row in candidates {
        if row.latitude.is_some() && row.longitude.is_some() {
            report.skipped += 1;
            continue;
        }

        let parsed: ActivityLocationJson = serde_json::from_str(&row.location).unwrap_or_default();
        let queries = build_queries(&parsed, &row.title);

        let mut chosen: Option<(String, f64, f64)> = None;
        for query in queries {
            let cache_key = query.to_lowercase();
            if let Some((lat, lon)) = cache.get(&cache_key).copied() {
                chosen = Some((query, lat, lon));
                break;
            }

            let coords = match location_service::search_locations_upstream(&query, 3).await {
                Ok(results) => results.first().map(|r| (r.latitude, r.longitude)),
                Err(_) => {
                    report.failed += 1;
                    chosen = None;
                    break;
                }
            };

            if let Some((lat, lon)) = coords {
                cache.insert(cache_key, (lat, lon));
                chosen = Some((query, lat, lon));
                break;
            }
        }

        let Some((_used_query, lat, lon)) = chosen else {
            warn!(
                "📍 No coords found for activity {} (title='{}')",
                row.activity_id, row.title
            );
            report.failed += 1;
            continue;
        };

        let updated = activity_repo::update_activity_geo(pool, &row.activity_id, lat, lon).await?;
        if updated > 0 {
            report.updated += 1;
        } else {
            report.failed += 1;
        }
    }

    info!(
        "📍 Activity geo backfill done: candidates={}, updated={}, skipped={}, failed={}",
        report.candidates, report.updated, report.skipped, report.failed
    );

    Ok(report)
}

fn build_queries(loc: &ActivityLocationJson, fallback_title: &str) -> Vec<String> {
    let mut parts = Vec::new();
    if let Some(v) = loc
        .venue_name
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        parts.push(v.to_string());
    }
    if let Some(v) = loc
        .city
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        parts.push(v.to_string());
    }
    if let Some(v) = loc
        .postal_code
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        parts.push(v.to_string());
    }
    if let Some(v) = loc
        .country
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        parts.push(v.to_string());
    }

    let mut queries = Vec::new();
    if !parts.is_empty() {
        queries.push(parts.join(" "));
    }

    if let Some(city) = loc
        .city
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        if let Some(country) = loc
            .country
            .as_ref()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
        {
            queries.push(format!("{} {}", city, country));
        }
        queries.push(city.to_string());
    }

    if queries.is_empty() {
        queries.push(fallback_title.to_string());
    }

    let mut seen = std::collections::HashSet::new();
    queries
        .into_iter()
        .filter(|q| seen.insert(q.to_lowercase()))
        .collect()
}
