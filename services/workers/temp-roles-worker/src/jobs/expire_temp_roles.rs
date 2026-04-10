use redis::AsyncCommands;
use sqlx::PgPool;
use tracing::{debug, info, warn};
use uuid::Uuid;

const REDIS_CHANNEL: &str = "sentinel:events";

#[derive(sqlx::FromRow)]
struct ExpiredRole {
    id: Uuid,
    guild_id: String,
    user_id: String,
    role_id: String,
}

/// Phase 4 B — Scan + emission Redis des roles temporaires expires.
///
/// Le worker ne peut PAS appeler `member.remove_role()` directement (pas de
/// connexion gateway Discord). Il publie un event sur `sentinel:events` que le
/// `community-bot` ecoute via `sentinel_shared::redis_listener` et execute le
/// retrait Discord local + DELETE de la ligne en DB.
///
/// Pour eviter les doublons, on peut soit :
///   - laisser le bot DELETE la ligne apres remove_role reussi (pattern actuel)
///   - SUPPRIMER ici et le bot ne touche que Discord
///
/// On garde l'ancien pattern (le bot DELETE) pour rester compatible avec les
/// flows existants. Le worker se contente de PUBLIER l'event.
pub async fn run(pool: &PgPool, redis: &redis::Client) -> Result<(), String> {
    let expired: Vec<ExpiredRole> = sqlx::query_as::<_, ExpiredRole>(
        "SELECT id, guild_id, user_id, role_id FROM temp_roles \
         WHERE expires_at <= NOW() \
         ORDER BY expires_at ASC \
         LIMIT 100",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("query expired temp_roles: {e}"))?;

    if expired.is_empty() {
        debug!("Aucun role temporaire expire");
        return Ok(());
    }

    let mut conn = redis
        .get_multiplexed_async_connection()
        .await
        .map_err(|e| format!("redis connect: {e}"))?;

    let mut published = 0u32;
    for role in &expired {
        let payload = serde_json::json!({
            "event": "temp_role_expire",
            "data": {
                "guild_id": role.guild_id,
                "user_id": role.user_id,
                "role_id": role.role_id,
            }
        });
        let serialized = match serde_json::to_string(&payload) {
            Ok(s) => s,
            Err(e) => {
                warn!(error = %e, "serialize event");
                continue;
            }
        };

        match conn.publish::<_, _, ()>(REDIS_CHANNEL, &serialized).await {
            Ok(_) => published += 1,
            Err(e) => warn!(role_id = %role.id, error = %e, "publish failed"),
        }
    }

    if published > 0 {
        info!(
            published,
            total = expired.len(),
            "Roles temporaires expires : events emis vers community-bot"
        );
    }

    Ok(())
}
