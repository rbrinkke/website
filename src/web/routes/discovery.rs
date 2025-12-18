use askama::Template;
use axum::{
    extract::{Query, State},
    response::Html,
    Extension,
};
use sqlx::SqlitePool;

use crate::services::discovery_service::{self, DiscoveryQuery};
use crate::web::middleware::auth::AuthenticatedUser;

#[derive(Template)]
#[template(path = "discovery.html")]
pub struct DiscoveryTemplate {
    pub users: Vec<crate::models::DiscoveryUserRow>,
    pub filters: discovery_service::AppliedFilters,
}

pub async fn discovery_handler(
    Extension(auth_user): Extension<AuthenticatedUser>,
    Query(query): Query<DiscoveryQuery>,
    State(pool): State<SqlitePool>,
) -> Html<String> {
    let data = discovery_service::build_discovery_page(&pool, &auth_user.id, &query)
        .await
        .unwrap_or(discovery_service::DiscoveryPageData {
            users: vec![],
            filters: discovery_service::AppliedFilters::default(),
        });
    let template = DiscoveryTemplate {
        users: data.users,
        filters: data.filters,
    };
    Html(template.render().unwrap())
}
