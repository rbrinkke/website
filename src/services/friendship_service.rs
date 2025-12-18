use sqlx::SqlitePool;
use uuid::Uuid;

use crate::database::friendship_commands_repo;

pub async fn create_friendship_command(
    pool: &SqlitePool,
    actor_user_id: &str,
    target_user_id: &str,
    action: &str,
) -> sqlx::Result<()> {
    let action = action.trim();
    if action != "request" && action != "cancel" && action != "accept" && action != "decline" {
        return Err(sqlx::Error::Protocol("invalid action".into()));
    }

    let id = Uuid::new_v4().to_string();
    friendship_commands_repo::insert_friendship_command(
        pool,
        friendship_commands_repo::NewFriendshipCommand {
            id: &id,
            actor_user_id,
            target_user_id,
            action,
            note: Some("website"),
        },
    )
    .await?;
    Ok(())
}
