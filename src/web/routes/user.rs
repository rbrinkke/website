use askama::Template;
use axum::Form;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect},
    Extension,
};
use sqlx::SqlitePool;
use tracing::warn;

use crate::services::friendship_service;
use crate::services::user_service;
use crate::services::user_summary_service;
use crate::web::middleware::auth::AuthenticatedUser;

#[derive(Template)]
#[template(path = "user.html")]
pub struct UserProfileTemplate {
    pub user: user_service::UserProfileView,
}

pub async fn user_profile_handler(
    Extension(_auth_user): Extension<AuthenticatedUser>,
    Path(user_id): Path<String>,
    State(pool): State<SqlitePool>,
) -> impl IntoResponse {
    let view = match user_service::load_user_profile_view(&pool, &user_id).await {
        Ok(v) => v,
        Err(e) => {
            warn!("User profile load failed for {}: {}", user_id, e);
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let Some(view) = view else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let template = UserProfileTemplate { user: view };
    Html(template.render().unwrap()).into_response()
}

#[derive(Template)]
#[template(path = "user_summary.html")]
pub struct UserSummaryTemplate {
    pub user: user_summary_service::UserSummaryView,
}

pub async fn user_summary_handler(
    Extension(auth_user): Extension<AuthenticatedUser>,
    Path(user_id): Path<String>,
    State(pool): State<SqlitePool>,
) -> impl IntoResponse {
    let view =
        match user_summary_service::load_user_summary_view(&pool, &auth_user.id, &user_id).await {
            Ok(v) => v,
            Err(e) => {
                warn!("User summary load failed for {}: {}", user_id, e);
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
        };

    let Some(view) = view else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let template = UserSummaryTemplate { user: view };
    Html(template.render().unwrap()).into_response()
}

#[derive(Debug, serde::Deserialize)]
pub struct FriendshipCommandForm {
    pub action: String, // request|cancel|accept|decline
    pub return_to: Option<String>,
}

pub async fn friendship_command_handler(
    Extension(auth_user): Extension<AuthenticatedUser>,
    Path(target_user_id): Path<String>,
    State(pool): State<SqlitePool>,
    Form(form): Form<FriendshipCommandForm>,
) -> impl IntoResponse {
    let action = form.action.as_str();
    let notice = match friendship_service::create_friendship_command(
        &pool,
        &auth_user.id,
        &target_user_id,
        action,
    )
    .await
    {
        Ok(_) => "ok",
        Err(e) => {
            warn!("Friendship command failed: {}", e);
            "error"
        }
    };

    let target = form
        .return_to
        .as_deref()
        .filter(|s| s.starts_with('/') && !s.starts_with("//") && !s.contains("://"))
        .unwrap_or("/discovery");

    let sep = if target.contains('?') { "&" } else { "?" };
    Redirect::to(&format!("{}{}notice={}", target, sep, notice)).into_response()
}
