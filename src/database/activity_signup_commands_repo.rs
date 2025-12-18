use sqlx::SqlitePool;

const SQL_INSERT_SIGNUP_COMMAND: &str = r#"
INSERT INTO activity_signup_commands (
  id,
  actor_user_id,
  activity_id,
  subject_user_id,
  action,
  note
) VALUES (?, ?, ?, ?, ?, ?)
"#;

#[allow(dead_code)]
pub struct NewActivitySignupCommand<'a> {
    pub id: &'a str,
    pub actor_user_id: &'a str,
    pub activity_id: &'a str,
    pub subject_user_id: &'a str,
    pub action: &'a str, // join|leave
    pub note: Option<&'a str>,
}

pub async fn insert_signup_command(
    pool: &SqlitePool,
    cmd: NewActivitySignupCommand<'_>,
) -> sqlx::Result<u64> {
    let res = sqlx::query(SQL_INSERT_SIGNUP_COMMAND)
        .bind(cmd.id)
        .bind(cmd.actor_user_id)
        .bind(cmd.activity_id)
        .bind(cmd.subject_user_id)
        .bind(cmd.action)
        .bind(cmd.note)
        .execute(pool)
        .await?;
    Ok(res.rows_affected())
}
