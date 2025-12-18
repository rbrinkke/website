use serde::Deserialize;
use serde::Serialize;
use tracing::warn;

#[derive(Debug, Serialize, Clone)]
pub struct LocationResult {
    pub id: String,
    pub name: String,
    pub description: String,
    pub latitude: f64,
    pub longitude: f64,
}

#[derive(Debug, Deserialize)]
struct Geo {
    lat: Option<f64>,
    lng: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct LocationHit {
    id: Option<String>,
    naam: Option<String>,
    name: Option<String>,
    weergave: Option<String>,
    description: Option<String>,
    lat: Option<f64>,
    lon: Option<f64>,
    latitude: Option<f64>,
    longitude: Option<f64>,
    #[serde(rename = "_geo")]
    geo: Option<Geo>,
}

#[derive(Debug, Deserialize)]
struct SearchResponse {
    hits: Option<Vec<LocationHit>>,
}

pub async fn search_locations_upstream(q: &str, limit: usize) -> Result<Vec<LocationResult>, ()> {
    let q = q.trim();
    if q.len() < 2 {
        return Ok(Vec::new());
    }

    let limit = limit.clamp(1, 20);
    let base_url =
        std::env::var("LOCATIE_API_URL").unwrap_or_else(|_| "http://127.0.0.1:8080".to_string());
    let host_header =
        std::env::var("LOCATIE_SERVICE_HOST").unwrap_or_else(|_| "locatie.localhost".to_string());
    let api_key = std::env::var("LOCATIE_API_KEY").ok();

    let url = format!("{}/search", base_url.trim_end_matches('/'));
    let client = reqwest::Client::new();

    let mut req = client
        .get(&url)
        .query(&[("q", q), ("limit", &limit.to_string())])
        .header("Host", host_header);

    if let Some(key) = api_key {
        req = req.header("x-api-key", key);
    }

    let resp = match req.send().await {
        Ok(r) => r,
        Err(e) => {
            warn!("📍 Locatie search upstream unreachable: {}", e);
            return Err(());
        }
    };

    if !resp.status().is_success() {
        warn!("📍 Locatie search upstream non-OK: {}", resp.status());
        return Err(());
    }

    let parsed: SearchResponse = match resp.json().await {
        Ok(data) => data,
        Err(e) => {
            warn!("📍 Locatie search upstream JSON parse failed: {}", e);
            return Err(());
        }
    };

    let hits = parsed.hits.unwrap_or_default();
    let results = hits
        .into_iter()
        .filter_map(|hit| {
            let geo_lat = hit.geo.as_ref().and_then(|g| g.lat);
            let geo_lng = hit.geo.as_ref().and_then(|g| g.lng);
            let lat = geo_lat.or(hit.lat).or(hit.latitude)?;
            let lon = geo_lng.or(hit.lon).or(hit.longitude)?;

            Some(LocationResult {
                id: hit.id.unwrap_or_default(),
                name: hit.naam.or(hit.name).unwrap_or_default(),
                description: hit.weergave.or(hit.description).unwrap_or_default(),
                latitude: lat,
                longitude: lon,
            })
        })
        .collect::<Vec<_>>();

    Ok(results)
}
