#[derive(Debug, Clone, sqlx::FromRow)]
pub struct UsersRow {
    pub name: Option<String>,
    pub profile_description: Option<String>,
    pub age: Option<i64>,
    pub gender: Option<String>,
    pub city: Option<String>,
    pub country: Option<String>,
    pub main_photo_url: Option<String>,
    pub profile_photos_extra: Option<String>,
    pub is_verified: Option<i64>,
    pub interests: Option<String>,
    pub subscription_level: Option<String>,
    pub activities_created_count: Option<i64>,
    pub activities_attended_count: Option<i64>,
    pub last_seen_at: Option<String>,
}
