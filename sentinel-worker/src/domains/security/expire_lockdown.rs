//! Phase 5G — Scanne `security_lockdown_active` pour les guilds dont
//! le lockdown a expire et publie un event Redis avec le JSON des
//! `saved_states` (overwrites originaux a restaurer).
//!
//! Le bot consume l'event, desserialise les overwrites et restaure les
//! permissions.

use sqlx::PgPool;
use tracing::{debug, info, warn};

const STREAM_KEY: &str = "sentinel:events";
const STREAM_MAXLEN: usize = 10_000;
const PAYLOAD_FIELD: &str = "payload";

#[derive(sqlx::FromRow)]
struct ExpiredLockdown {
    guild_id: String,
    saved_states: serde_json::Value,
}

pub async fn run(pool: &PgPool, redis: &redis::Client) -> Result<(), String> {
    let candidates: Vec<ExpiredLockdown> = sqlx::query_as(
        "SELECT guild_id, saved_states \
         FROM security_lockdown_active \
         WHERE expires_at < NOW() \
         ORDER BY expires_at ASC LIMIT 50",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("query expired lockdowns: {e}"))?;

    if candidates.is_empty() {
        debug!("Aucun lockdown expire");
        return Ok(());
    }

    let mut conn = redis
        .get_multiplexed_async_connection()
        .await
        .map_err(|e| format!("redis connect: {e}"))?;

    let mut reverted = 0u32;
    for lk in &candidates {
        if !crate::common::is_worker_enabled(pool, &lk.guild_id, "security-bot").await {
            continue;
        }
        // Claim atomique : DELETE avec garde expires_at.
        let deleted = sqlx::query(
            "DELETE FROM security_lockdown_active \
             WHERE guild_id = $1 AND expires_at < NOW()",
        )
        .bind(&lk.guild_id)
        .execute(pool)
        .await
        .map_err(|e| format!("claim expired lockdown: {e}"))?;
        if deleted.rows_affected() == 0 {
            continue;
        }

        let payload = serde_json::json!({
            "event": "lockdown_expired",
            "data": {
                "guild_id": lk.guild_id,
                "saved_states": lk.saved_states,
            }
        });
        let res: redis::RedisResult<String> = redis::cmd("XADD")
            .arg(STREAM_KEY)
            .arg("MAXLEN")
            .arg("~")
            .arg(STREAM_MAXLEN)
            .arg("*")
            .arg(PAYLOAD_FIELD)
            .arg(payload.to_string())
            .query_async(&mut conn)
            .await;
        if let Err(e) = res {
            warn!(error = %e, guild = %lk.guild_id, "XADD lockdown_expired echoue");
        }
        reverted += 1;
    }

    if reverted > 0 {
        info!(reverted, "Lockdowns expires -> events publies pour restauration");
    }
    Ok(())
}
