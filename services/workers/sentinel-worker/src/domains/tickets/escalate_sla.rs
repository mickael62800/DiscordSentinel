//! Phase 5I — Escalade SLA pour TOUS les tickets (sauf appel_sanction
//! qui est gere par appeal_sla::escalate_appeal_sla).
//!
//! Avant : tickets/mod.rs avait une boucle 5min qui scannait l'API
//! tickets et utilisait un SlaTracker RAM. Si le bot redemarrait,
//! les timestamps de premiere reponse etaient perdus.
//!
//! Maintenant : la donnee est deja en DB (tickets.first_response_at),
//! le worker scanne et publie un event ticket_sla_escalated. Le bot
//! consume et poste le message d'avertissement dans le channel.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use tracing::{debug, info, warn};
use uuid::Uuid;

const STREAM_KEY: &str = "sentinel:events";
const STREAM_MAXLEN: usize = 10_000;
const PAYLOAD_FIELD: &str = "payload";
const DEFAULT_SLA_ESCALATION_MINUTES: i64 = 60;

#[derive(sqlx::FromRow)]
struct CandidateTicket {
    id: Uuid,
    server: String,
    channel_id: Option<String>,
    created_at: DateTime<Utc>,
}

pub async fn run(pool: &PgPool, redis: &redis::Client) -> Result<(), String> {
    let timeouts = load_escalation_timeouts(pool).await;

    // Tickets pas encore repondus + pas encore escalades + categorie != appel.
    // Filtre grossier > 1 min, on affine par guild apres.
    let candidates: Vec<CandidateTicket> = sqlx::query_as(
        "SELECT id, server, channel_id, created_at \
         FROM tickets \
         WHERE category != 'appel_sanction' \
           AND status IN ('open', 'assigned') \
           AND escalated_at IS NULL \
           AND first_response_at IS NULL \
           AND created_at < NOW() - INTERVAL '1 minute' \
         ORDER BY created_at ASC \
         LIMIT 100",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("query candidate tickets: {e}"))?;

    if candidates.is_empty() {
        debug!("Aucun ticket non-appel en attente d'escalade");
        return Ok(());
    }

    let mut conn = redis
        .get_multiplexed_async_connection()
        .await
        .map_err(|e| format!("redis connect: {e}"))?;

    let now = Utc::now();
    let mut escalated = 0u32;

    for t in &candidates {
        if !crate::common::is_worker_enabled(pool, &t.server, "ticket-bot").await {
            continue;
        }
        let escalation_minutes = timeouts
            .get(&t.server)
            .copied()
            .unwrap_or(DEFAULT_SLA_ESCALATION_MINUTES);
        if escalation_minutes <= 0 {
            continue;
        }
        let age_minutes = (now - t.created_at).num_minutes();
        if age_minutes < escalation_minutes {
            continue;
        }

        // UPDATE atomique avec garde + bumps priorite a high.
        let updated = sqlx::query(
            "UPDATE tickets SET escalated_at = NOW(), priority = 'high', updated_at = NOW() \
             WHERE id = $1 AND escalated_at IS NULL",
        )
        .bind(t.id)
        .execute(pool)
        .await
        .map_err(|e| format!("mark escalated: {e}"))?;
        if updated.rows_affected() == 0 {
            continue;
        }

        let payload = serde_json::json!({
            "event": "ticket_sla_escalated",
            "data": {
                "ticket_id": t.id.to_string(),
                "guild_id": t.server,
                "channel_id": t.channel_id,
                "age_minutes": age_minutes,
                "escalation_minutes": escalation_minutes,
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
            warn!(error = %e, ticket_id = %t.id, "XADD ticket_sla_escalated echoue");
        }
        escalated += 1;
    }

    if escalated > 0 {
        info!(escalated, "Tickets escalades SLA -> events publies");
    }
    Ok(())
}

async fn load_escalation_timeouts(pool: &PgPool) -> HashMap<String, i64> {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT guild_id, config_value FROM bot_guild_config \
         WHERE bot_name = 'ticket-bot' AND config_key = 'sla_escalation_minutes'",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    rows.into_iter()
        .filter_map(|(g, v)| v.parse::<i64>().ok().map(|n| (g, n)))
        .collect()
}
