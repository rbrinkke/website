#[derive(Debug, Clone, sqlx::FromRow)]
pub struct UserProfilesRow {
    pub search_radius: i64,
    pub filter_min_age: Option<i64>,
    pub filter_max_age: Option<i64>,
    pub filter_gender: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
}
