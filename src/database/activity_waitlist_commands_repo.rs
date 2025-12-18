use sqlx::SqlitePool;

const SQL_INSERT_WAITLIST_COMMAND: &str = r#"
INSERT INTO activity_waitlist_commands (
  id,
  actor_user_id,
  activity_id,
  subject_user_id,
  action,
  priority,
  note
) VALUES (?, ?, ?, ?, ?, ?, ?)
"#;

#[allow(dead_code)]
pub struct NewActivityWaitlistCommand<'a> {
    pub id: &'a str,
    pub actor_user_id: &'a str,
    pub activity_id: &'a str,
    pub subject_user_id: &'a str,
    pub action: &'a str, // set_waitlisted|remove_waitlist|set_priority
    pub priority: Option<i64>,
    pub note: Option<&'a str>,
}

pub async fn insert_waitlist_command(
    pool: &SqlitePool,
    cmd: NewActivityWaitlistCommand<'_>,
) -> sqlx::Result<u64> {
    let res = sqlx::query(SQL_INSERT_WAITLIST_COMMAND)
        .bind(cmd.id)
        .bind(cmd.actor_user_id)
        .bind(cmd.activity_id)
        .bind(cmd.subject_user_id)
        .bind(cmd.action)
        .bind(cmd.priority)
        .bind(cmd.note)
        .execute(pool)
        .await?;
    Ok(res.rows_affected())
}
