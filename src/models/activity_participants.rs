// High-churn activity participant rows (waitlist/registered etc).
#[allow(dead_code)]
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ActivityParticipantsRow {
    pub activity_id: String,
    pub user_id: String,
    pub name: Option<String>,
    pub photo_url: Option<String>,
    pub role: Option<String>,
    pub participation_status: Option<String>,
    pub attendance_status: Option<String>,
    pub joined_at: Option<String>,
    pub updated_at: Option<String>,
    pub is_deleted: Option<i64>,
}
