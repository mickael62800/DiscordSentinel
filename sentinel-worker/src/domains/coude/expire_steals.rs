//! Phase 5 — Expire les tentatives de vol /voler dont la fenetre de
//! defense (60s par defaut) est ecoulee.
//!
//! Pattern aligne sur `appeal_sla::escalate_appeal_sla` :
//!   1. SELECT des `coude_steal_attempts` `pending` avec
//!      `expires_at < NOW()` (limit 100, ordre FIFO).
//!   2. UPDATE atomique `status='expired'` AVANT publication Redis
//!      (garde `WHERE status='pending'` pour idempotence si plusieurs
//!      workers tournent — pas le cas en pratique mais defensif).
//!   3. XADD sur `sentinel:events` event `coude:steal_expired` avec le
//!      payload necessaire au bot pour resoudre la tentative AFK :
//!      thief_id, target_id, guild_id, message_id, channel_id, attempt_id.
//!   4. Le bot a un consumer Redis pour cet event ; il execute la
//!      resolution avec malus AFK (cf modules/coude/commands/voler.rs)
//!      puis PATCH /api/coude/steals/{id}/resolved.

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use tracing::{debug, info, warn};
use uuid::Uuid;

const STREAM_KEY: &str = "sentinel:events";
const STREAM_MAXLEN: usize = 10_000;
const PAYLOAD_FIELD: &str = "payload";

#[derive(sqlx::FromRow)]
struct ExpiredAttempt {
    id: Uuid,
    guild_id: String,
    thief_id: String,
    target_id: String,
    message_id: String,
    channel_id: String,
    expires_at: DateTime<Utc>,
}

pub async fn run(pool: &PgPool, redis: &redis::Client) -> Result<(), String> {
    let candidates: Vec<ExpiredAttempt> = sqlx::query_as(
        "SELECT id, guild_id, thief_id, target_id, message_id, channel_id, expires_at \
         FROM coude_steal_attempts \
         WHERE status = 'pending' AND expires_at < NOW() \
         ORDER BY expires_at ASC LIMIT 100",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("query expired steals: {e}"))?;

    if candidates.is_empty() {
        debug!("Aucune tentative de vol expiree");
        return Ok(());
    }

    let mut conn = redis
        .get_multiplexed_async_connection()
        .await
        .map_err(|e| format!("redis connect: {e}"))?;

    let mut expired_count = 0u32;

    for att in &candidates {
        if !crate::common::is_worker_enabled(pool, &att.guild_id, "coude-bot").await {
            continue;
        }
        // Claim atomique : passe pending -> expired. Si une autre instance
        // ou la victime a defense entre-temps, rows_affected = 0, on skip.
        let updated = sqlx::query(
            "UPDATE coude_steal_attempts \
             SET status = 'expired', resolved_at = NOW() \
             WHERE id = $1 AND status = 'pending'",
        )
        .bind(att.id)
        .execute(pool)
        .await
        .map_err(|e| format!("claim expired: {e}"))?;

        if updated.rows_affected() == 0 {
            continue;
        }

        let payload = serde_json::json!({
            "event": "coude:steal_expired",
            "data": {
                "attempt_id": att.id.to_string(),
                "guild_id": att.guild_id,
                "thief_id": att.thief_id,
                "target_id": att.target_id,
                "message_id": att.message_id,
                "channel_id": att.channel_id,
                "expires_at": att.expires_at.to_rfc3339(),
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

        match res {
            Ok(_) => {
                expired_count += 1;
            }
            Err(e) => {
                // Redis HS : on a deja UPDATE le row, donc on ne peut pas
                // re-essayer. On log, le bot ne resoudra pas ce vol — la
                // victime echappe par defaut. Acceptable degradation.
                warn!(error = %e, attempt_id = %att.id, "XADD coude:steal_expired echoue");
            }
        }
    }

    if expired_count > 0 {
        info!(
            expired = expired_count,
            "Tentatives de vol expirees -> events publies pour resolution AFK"
        );
    }
    Ok(())
}
