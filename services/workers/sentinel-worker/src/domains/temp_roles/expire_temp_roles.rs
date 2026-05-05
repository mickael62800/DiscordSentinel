use sqlx::PgPool;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Phase 5B : XADD sur la stream `sentinel:events` (remplace pub/sub PUBLISH).
/// Doit rester synchronise avec `bots/shared/src/event_bus.rs`.
const STREAM_KEY: &str = "sentinel:events";
const STREAM_MAXLEN: usize = 10_000;
const PAYLOAD_FIELD: &str = "payload";

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
/// connexion gateway Discord). Il emet un event via XADD sur la stream
/// `sentinel:events` (Phase 5B) que le `community-bot` consomme pour executer
/// le retrait Discord local + DELETE de la ligne en DB.
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
        // Guard top-level enabled : si la guild a desactive le module
        // temp_roles, on ne publie pas l'event (le role expire reste
        // en DB, le bot le retirera quand le module sera reactive).
        if !crate::common::is_worker_enabled(pool, &role.guild_id, "temp_roles").await {
            continue;
        }
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

        let res: redis::RedisResult<String> = redis::cmd("XADD")
            .arg(STREAM_KEY)
            .arg("MAXLEN")
            .arg("~")
            .arg(STREAM_MAXLEN)
            .arg("*")
            .arg(PAYLOAD_FIELD)
            .arg(&serialized)
            .query_async(&mut conn)
            .await;
        match res {
            Ok(_) => published += 1,
            Err(e) => warn!(role_id = %role.id, error = %e, "XADD failed"),
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
