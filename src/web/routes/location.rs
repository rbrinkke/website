use crate::services::location_service;
use axum::{extract::Query, http::StatusCode, response::IntoResponse, Json};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct LocationSearchQuery {
    q: Option<String>,
    limit: Option<usize>,
}

pub async fn search_locations(Query(query): Query<LocationSearchQuery>) -> impl IntoResponse {
    let q = match query.q.as_ref().map(|s| s.trim()).filter(|s| s.len() >= 2) {
        Some(v) => v,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(Vec::<location_service::LocationResult>::new()),
            )
        }
    };

    let limit = query.limit.unwrap_or(8).min(20);
    match location_service::search_locations_upstream(q, limit).await {
        Ok(results) => (StatusCode::OK, Json(results)),
        Err(_) => (
            StatusCode::BAD_GATEWAY,
            Json(Vec::<location_service::LocationResult>::new()),
        ),
    }
}
