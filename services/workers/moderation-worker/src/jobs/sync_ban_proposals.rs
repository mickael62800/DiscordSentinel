use sqlx::PgPool;
use tracing::{debug, info};
use uuid::Uuid;

use sentinel_worker_common::is_worker_enabled;

#[derive(sqlx::FromRow)]
struct ZeroPointsUser {
    guild_id: String,
    user_id: String,
    username: String,
}

/// Scanne les utilisateurs a 0 points de conduite et cree une proposition de ban
/// si aucune infraction "ban" n'existe deja pour eux.
pub async fn run(pool: &PgPool) -> Result<(), String> {
    let users: Vec<ZeroPointsUser> = sqlx::query_as::<_, ZeroPointsUser>(
        "SELECT ucp.guild_id, ucp.user_id, ucp.username \
         FROM user_conduct_points ucp \
         WHERE ucp.points <= 0 \
         AND NOT EXISTS ( \
             SELECT 1 FROM infractions i \
             WHERE i.guild_id = ucp.guild_id \
             AND i.user_id = ucp.user_id \
             AND i.action = 'ban' \
             AND i.reason LIKE 'Points de conduite%' \
         )",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Query zero points users: {e}"))?;

    if users.is_empty() {
        debug!("Aucun utilisateur a 0 points sans proposition de ban");
        return Ok(());
    }

    let mut count = 0u64;

    for user in &users {
        if !is_worker_enabled(pool, &user.guild_id, "moderation-worker").await {
            continue;
        }

        sqlx::query(
            "INSERT INTO infractions (id, guild_id, channel_id, user_id, username, message_id, content, flags, score, action, reason, duration, created_at) \
             VALUES ($1, $2, '', $3, $4, '', '', '{}'::jsonb, 0, 'ban', $5, NULL, NOW())",
        )
        .bind(Uuid::new_v4())
        .bind(&user.guild_id)
        .bind(&user.user_id)
        .bind(&user.username)
        .bind("Points de conduite tombes a 0")
        .execute(pool)
        .await
        .map_err(|e| format!("Insert ban proposal: {e}"))?;

        count += 1;
    }

    info!(count, "Propositions de ban creees pour utilisateurs a 0 points");

    Ok(())
}
