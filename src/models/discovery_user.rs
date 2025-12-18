// View-model row for the Discovery grid (users + computed friend flag).
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct DiscoveryUserRow {
    pub user_id: String,
    pub name: Option<String>,
    pub city: Option<String>,
    pub main_photo_url: Option<String>,
    pub is_verified: Option<i64>,
    pub age: Option<i64>,
    pub gender: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    #[sqlx(skip)]
    pub distance_km: Option<f64>,
    pub is_friend: Option<i64>,
}
