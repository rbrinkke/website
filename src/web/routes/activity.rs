use askama::Template;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect},
    Extension, Form,
};
use serde::Deserialize;
use sqlx::SqlitePool;
use tracing::warn;

use crate::services::activity_detail_service::{self, ActivityDetailQuery};
use crate::services::activity_summary_service;
use crate::web::middleware::auth::AuthenticatedUser;

#[derive(Template)]
#[template(path = "activity.html")]
pub struct ActivityDetailTemplate {
    pub activity: activity_detail_service::ActivityDetailView,
    pub can_manage_activity: bool,
}

pub async fn activity_detail_handler(
    Extension(_auth_user): Extension<AuthenticatedUser>,
    Path(activity_id): Path<String>,
    Query(query): Query<ActivityDetailQuery>,
    State(pool): State<SqlitePool>,
) -> impl IntoResponse {
    // Detail page is retired; keep the route for backwards compatibility and deep links.
    let _ = (activity_id, query, pool);
    Redirect::to("/activities").into_response()
}

#[derive(Template)]
#[template(path = "activity_summary.html")]
pub struct ActivitySummaryTemplate {
    pub summary: activity_summary_service::ActivitySummaryView,
}

#[derive(Debug, Deserialize, Default)]
pub struct ActivitySummaryQuery {
    pub tab: Option<String>,
    pub return_to: Option<String>,
}

pub async fn activity_summary_handler(
    Extension(_auth_user): Extension<AuthenticatedUser>,
    Path(activity_id): Path<String>,
    Query(query): Query<ActivitySummaryQuery>,
    State(pool): State<SqlitePool>,
) -> impl IntoResponse {
    let view = match activity_summary_service::load_activity_summary_view(
        &pool,
        &activity_id,
        query.tab.clone(),
        query.return_to.clone(),
    )
    .await
    {
        Ok(v) => v,
        Err(e) => {
            warn!("Activity summary load failed for {}: {}", activity_id, e);
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let Some(view) = view else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let template = ActivitySummaryTemplate { summary: view };
    Html(template.render().unwrap()).into_response()
}

#[derive(Debug, Deserialize)]
pub struct SignupCommandForm {
    pub action: String, // join|leave
    pub subject_user_id: Option<String>,
    pub return_to: Option<String>,
}

pub async fn activity_signup_command_handler(
    Extension(auth_user): Extension<AuthenticatedUser>,
    Path(activity_id): Path<String>,
    State(pool): State<SqlitePool>,
    Form(form): Form<SignupCommandForm>,
) -> impl IntoResponse {
    let subject = form.subject_user_id.as_deref().unwrap_or(&auth_user.id);
    let action = form.action.as_str();

    if action != "join" && action != "leave" {
        return StatusCode::BAD_REQUEST.into_response();
    }

    let notice = match activity_detail_service::create_signup_command(
        &pool,
        &auth_user.id,
        &activity_id,
        subject,
        action,
    )
    .await
    {
        Ok(_) => match action {
            "join" => {
                if subject == auth_user.id {
                    "join_ok"
                } else {
                    "owner_join_ok"
                }
            }
            "leave" => {
                if subject == auth_user.id {
                    "leave_ok"
                } else {
                    "owner_leave_ok"
                }
            }
            _ => "ok",
        },
        Err(e) => {
            warn!("Signup command failed: {}", e);
            "error"
        }
    };

    if let Some(target) = form.return_to.as_deref().and_then(sanitize_return_to) {
        let sep = if target.contains('?') { "&" } else { "?" };
        return Redirect::to(&format!("{}{}notice={}", target, sep, notice)).into_response();
    }

    Redirect::to(&format!("/activities/{}?notice={}", activity_id, notice)).into_response()
}

#[derive(Debug, Deserialize)]
pub struct WaitlistCommandForm {
    pub action: String, // set_waitlisted|remove_waitlist|set_priority
    pub subject_user_id: Option<String>,
    pub priority: Option<i64>,
    pub return_to: Option<String>,
}

pub async fn activity_waitlist_command_handler(
    Extension(auth_user): Extension<AuthenticatedUser>,
    Path(activity_id): Path<String>,
    State(pool): State<SqlitePool>,
    Form(form): Form<WaitlistCommandForm>,
) -> impl IntoResponse {
    let action = form.action.as_str();
    if action != "set_waitlisted" && action != "remove_waitlist" && action != "set_priority" {
        return StatusCode::BAD_REQUEST.into_response();
    }
    if action == "set_priority" && form.priority.is_none() {
        return StatusCode::BAD_REQUEST.into_response();
    }

    let subject = form.subject_user_id.as_deref().unwrap_or(&auth_user.id);
    let notice = match activity_detail_service::create_waitlist_command(
        &pool,
        &auth_user.id,
        &activity_id,
        subject,
        action,
        form.priority,
    )
    .await
    {
        Ok(_) => match action {
            "set_waitlisted" => "waitlist_set_ok",
            "remove_waitlist" => "waitlist_removed_ok",
            "set_priority" => "waitlist_priority_ok",
            _ => "ok",
        },
        Err(e) => {
            warn!("Waitlist command failed: {}", e);
            "error"
        }
    };

    if let Some(target) = form.return_to.as_deref().and_then(sanitize_return_to) {
        let sep = if target.contains('?') { "&" } else { "?" };
        return Redirect::to(&format!("{}{}notice={}", target, sep, notice)).into_response();
    }

    Redirect::to(&format!("/activities/{}?notice={}", activity_id, notice)).into_response()
}

fn sanitize_return_to(value: &str) -> Option<&str> {
    let v = value.trim();
    if !v.starts_with('/') {
        return None;
    }
    if v.starts_with("//") || v.contains("://") {
        return None;
    }
    Some(v)
}
