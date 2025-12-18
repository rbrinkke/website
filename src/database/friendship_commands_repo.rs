use sqlx::SqlitePool;

pub struct NewFriendshipCommand<'a> {
    pub id: &'a str,
    pub actor_user_id: &'a str,
    pub target_user_id: &'a str,
    pub action: &'a str, // request|cancel|accept|decline
    pub note: Option<&'a str>,
}

const SQL_INSERT_FRIENDSHIP_COMMAND: &str = r#"
INSERT INTO friendship_commands (
  id,
  actor_user_id,
  target_user_id,
  action,
  note
) VALUES (?1, ?2, ?3, ?4, ?5)
"#;

pub async fn insert_friendship_command(
    pool: &SqlitePool,
    cmd: NewFriendshipCommand<'_>,
) -> sqlx::Result<()> {
    sqlx::query(SQL_INSERT_FRIENDSHIP_COMMAND)
        .bind(cmd.id)
        .bind(cmd.actor_user_id)
        .bind(cmd.target_user_id)
        .bind(cmd.action)
        .bind(cmd.note)
        .execute(pool)
        .await?;
    Ok(())
}
