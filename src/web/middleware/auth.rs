use axum::{
    extract::{Request, State},
    http::header,
    middleware::Next,
    response::Response,
};
use base64::{engine::general_purpose, Engine as _};
use serde::Deserialize;
use sqlx::SqlitePool;

use crate::database::current_user_repo;

#[derive(Clone, Debug)]
pub struct AuthenticatedUser {
    pub id: String,
}

#[derive(Deserialize)]
struct JwtPayload {
    sub: String,
}

pub async fn require_auth(
    State(pool): State<SqlitePool>,
    mut request: Request,
    next: Next,
) -> Response {
    // Extract cookies from request
    let token = request
        .headers()
        .get(header::COOKIE)
        .and_then(|hv| hv.to_str().ok())
        .and_then(|cookies| {
            cookies
                .split("; ")
                .find(|c| c.starts_with("access_token="))
                .and_then(|c| c.strip_prefix("access_token="))
        });

    if let Some(token) = token {
        // Parse JWT payload (middle part)
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() == 3 {
            if let Ok(payload_bytes) = general_purpose::URL_SAFE_NO_PAD.decode(parts[1]) {
                if let Ok(payload) = serde_json::from_slice::<JwtPayload>(&payload_bytes) {
                    // Inject user id into request extensions
                    request
                        .extensions_mut()
                        .insert(AuthenticatedUser { id: payload.sub });

                    return next.run(request).await;
                }
            }
        }
    }

    // Fallback for offline/local usage: use the current_user table
    if let Ok(Some(user_id)) = current_user_repo::load_current_user_id(&pool).await {
        request
            .extensions_mut()
            .insert(AuthenticatedUser { id: user_id });
        return next.run(request).await;
    }

    // No valid token or parse error, return 401
    Response::builder()
        .status(401)
        .body(axum::body::Body::from("Unauthorized - Please login"))
        .unwrap()
}
