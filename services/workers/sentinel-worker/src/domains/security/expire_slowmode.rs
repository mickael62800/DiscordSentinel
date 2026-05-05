//! Phase 5H — Scanne `security_slowmode_active` pour les guilds dont
//! le slowmode anti-raid a expire et publie un event Redis.

use sqlx::PgPool;
use tracing::{debug, info, warn};

const STREAM_KEY: &str = "sentinel:events";
const STREAM_MAXLEN: usize = 10_000;
const PAYLOAD_FIELD: &str = "payload";

#[derive(sqlx::FromRow)]
struct ExpiredSlowmode {
    guild_id: String,
    previous_states: serde_json::Value,
}

pub async fn run(pool: &PgPool, redis: &redis::Client) -> Result<(), String> {
    let candidates: Vec<ExpiredSlowmode> = sqlx::query_as(
        "SELECT guild_id, previous_states \
         FROM security_slowmode_active \
         WHERE expires_at < NOW() \
         ORDER BY expires_at ASC LIMIT 50",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("query expired slowmode: {e}"))?;

    if candidates.is_empty() {
        debug!("Aucun slowmode expire");
        return Ok(());
    }

    let mut conn = redis
        .get_multiplexed_async_connection()
        .await
        .map_err(|e| format!("redis connect: {e}"))?;

    let mut reverted = 0u32;
    for s in &candidates {
        if !crate::common::is_worker_enabled(pool, &s.guild_id, "security-bot").await {
            continue;
        }
        let deleted = sqlx::query(
            "DELETE FROM security_slowmode_active \
             WHERE guild_id = $1 AND expires_at < NOW()",
        )
        .bind(&s.guild_id)
        .execute(pool)
        .await
        .map_err(|e| format!("claim expired slowmode: {e}"))?;
        if deleted.rows_affected() == 0 {
            continue;
        }

        let payload = serde_json::json!({
            "event": "slowmode_expired",
            "data": {
                "guild_id": s.guild_id,
                "previous_states": s.previous_states,
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
            warn!(error = %e, guild = %s.guild_id, "XADD slowmode_expired echoue");
        }
        reverted += 1;
    }

    if reverted > 0 {
        info!(reverted, "Slowmodes expires -> events publies");
    }
    Ok(())
}
