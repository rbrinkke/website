#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CurrentUserRow {
    pub user_id: String,
}
