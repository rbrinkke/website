use axum::{
    extract::Path,
    http::{header, HeaderMap, StatusCode},
    response::Response,
};
use tracing::error;

#[derive(serde::Deserialize)]
struct ImageMeta {
    url: String,
}

pub async fn image_proxy(
    Path(image_id): Path<String>,
    headers: HeaderMap,
) -> Result<Response, StatusCode> {
    println!("🖼️ Image proxy request for: {}", image_id);

    // 1. Haal JWT token uit cookie header
    let cookies = headers
        .get(header::COOKIE)
        .and_then(|hv| hv.to_str().ok())
        .unwrap_or("");

    let token = cookies
        .split("; ")
        .find_map(|cookie| {
            cookie
                .strip_prefix("access_token=")
                .map(|token| token.to_string())
        })
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // 2. Call image-api for Metadata
    let client = reqwest::Client::new();
    let base_url =
        std::env::var("IMAGE_API_URL").unwrap_or_else(|_| "http://localhost:8004".to_string());

    // Stap A: Haal metadata op
    let meta_url = format!("{}/api/v1/images/{}?size=medium", base_url, image_id);
    println!("📡 Fetching metadata: {}", meta_url);

    let meta_resp = client
        .get(&meta_url)
        .header("Host", "image.localhost")
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| {
            println!("❌ Metadata request failed: {}", e);
            error!("Metadata request failed: {}", e);
            StatusCode::BAD_GATEWAY
        })?;

    if !meta_resp.status().is_success() {
        println!("❌ Image API returned: {}", meta_resp.status());
        return Err(StatusCode::NOT_FOUND);
    }

    let meta: ImageMeta = meta_resp.json().await.map_err(|e| {
        println!("❌ Failed to parse metadata JSON: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Stap B: Haal de echte content op
    // De url in meta is bijv "/storage/users/..."
    // We plakken die achter de base_url. Let op dubbele slashes.
    let content_path = meta.url.trim_start_matches('/');
    let content_url = format!("{}/{}", base_url.trim_end_matches('/'), content_path);

    println!("📥 Fetching bytes from: {}", content_url);
    let content_resp = client
        .get(&content_url)
        .header("Host", "image.localhost")
        // Sommige storage endpoints hebben ook auth nodig
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| {
            println!("❌ Content request failed: {}", e);
            StatusCode::BAD_GATEWAY
        })?;

    if !content_resp.status().is_success() {
        println!("❌ Content download returned: {}", content_resp.status());
        return Err(StatusCode::NOT_FOUND);
    }

    let content_type = content_resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("image/jpeg")
        .to_string();

    println!("📦 Response Content-Type: {}", content_type);

    let bytes = content_resp.bytes().await.map_err(|e| {
        println!("❌ Failed to read content body: {}", e);
        StatusCode::BAD_GATEWAY
    })?;

    println!("✅ Got {} bytes from storage", bytes.len());

    // 4. Return image met proper headers
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", content_type)
        .header("Cache-Control", "public, max-age=3600")
        .body(axum::body::Body::from(bytes))
        .unwrap())
}
