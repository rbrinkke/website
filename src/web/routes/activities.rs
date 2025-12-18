use askama::Template;
use axum::{
    extract::{Query, State},
    response::Html,
    Extension,
};
use sqlx::SqlitePool;

use crate::services::activities_service::{self, ActivitiesQuery};
use crate::web::middleware::auth::AuthenticatedUser;

#[derive(Template)]
#[template(path = "activities.html")]
pub struct ActivitiesTemplate {
    pub activities: Vec<activities_service::ActivityCardView>,
    pub filters: activities_service::AppliedActivityFilters,
    pub interest_options: Vec<activities_service::InterestOptionView>,
}

pub async fn activities_handler(
    Extension(auth_user): Extension<AuthenticatedUser>,
    Query(query): Query<ActivitiesQuery>,
    State(pool): State<SqlitePool>,
) -> Html<String> {
    let data = activities_service::build_activities_page(&pool, &auth_user.id, &query)
        .await
        .unwrap_or(activities_service::ActivitiesPageData {
            tab: activities_service::ActivitiesTab::Discover,
            activities: vec![],
            filters: activities_service::AppliedActivityFilters {
                tab: "discover".to_string(),
                radius_km: 25,
                ..activities_service::AppliedActivityFilters::default()
            },
            interest_options: vec![],
        });

    let template = ActivitiesTemplate {
        activities: data.activities,
        filters: data.filters,
        interest_options: data.interest_options,
    };
    Html(template.render().unwrap())
}
