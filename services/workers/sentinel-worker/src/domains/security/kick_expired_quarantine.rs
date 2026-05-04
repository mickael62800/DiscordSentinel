//! Phase 5F — Scanne `security_quarantine_pending` pour les rows
//! expires et publie un event `quarantine_expired` que le bot
//! consume pour kicker.
//!
//! UPDATE+DELETE atomiques avec garde sur expires_at pour idempotence
//! si plusieurs workers tournent.

use sqlx::PgPool;
use tracing::{debug, info, warn};

const STREAM_KEY: &str = "sentinel:events";
const STREAM_MAXLEN: usize = 10_000;
const PAYLOAD_FIELD: &str = "payload";

#[derive(sqlx::FromRow)]
struct ExpiredQuarantine {
    guild_id: String,
    user_id: String,
}

pub async fn run(pool: &PgPool, redis: &redis::Client) -> Result<(), String> {
    let candidates: Vec<ExpiredQuarantine> = sqlx::query_as(
        "SELECT guild_id, user_id \
         FROM security_quarantine_pending \
         WHERE expires_at < NOW() \
         ORDER BY expires_at ASC LIMIT 100",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("query expired quarantine: {e}"))?;

    if candidates.is_empty() {
        debug!("Aucune quarantaine expiree");
        return Ok(());
    }

    let mut conn = redis
        .get_multiplexed_async_connection()
        .await
        .map_err(|e| format!("redis connect: {e}"))?;

    let mut kicked = 0u32;
    for q in &candidates {
        // Claim atomique : DELETE avec garde sur expires_at. Si une
        // autre instance ou le bot a deja retire l'entree (validation
        // captcha entre-temps), rows_affected = 0, on skip.
        let deleted = sqlx::query(
            "DELETE FROM security_quarantine_pending \
             WHERE guild_id = $1 AND user_id = $2 AND expires_at < NOW()",
        )
        .bind(&q.guild_id)
        .bind(&q.user_id)
        .execute(pool)
        .await
        .map_err(|e| format!("claim expired: {e}"))?;
        if deleted.rows_affected() == 0 {
            continue;
        }

        let payload = serde_json::json!({
            "event": "quarantine_expired",
            "data": {
                "guild_id": q.guild_id,
                "user_id": q.user_id,
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
            warn!(error = %e, guild = %q.guild_id, user = %q.user_id, "XADD quarantine_expired echoue");
        }
        kicked += 1;
    }

    if kicked > 0 {
        info!(kicked, "Quarantaines expirees -> events publies pour kick");
    }
    Ok(())
}
