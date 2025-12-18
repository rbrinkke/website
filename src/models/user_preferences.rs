#[derive(Debug, Clone, sqlx::FromRow)]
pub struct UserPreferencesRow {
    pub search_radius: i64,
    pub filter_min_age: Option<i64>,
    pub filter_max_age: Option<i64>,
    pub filter_gender: Option<String>,
    pub search_latitude: Option<f64>,
    pub search_longitude: Option<f64>,
}
