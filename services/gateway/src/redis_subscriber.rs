//! Phase 5B — Live tail Redis Streams pour le relay WebSocket.
//!
//! Le gateway lit en XREAD `$` la stream `sentinel:events` sans consumer group.
//! Semantique fire-and-forget preservee (identique a l'ancien pub/sub) :
//! - Si le gateway est down, les events ne sont PAS rejoues au redemarrage
//!   (on demarre au "dernier ID" au moment de la reconnexion).
//! - Si un client WS est deconnecte, ses events sont perdus cote client.
//!
//! Cette semantique est volontaire pour un dashboard temps reel : pas de
//! rattrapage de 1000 events obsoletes au redemarrage, juste la suite.
//!
//! Pour les consumers durables (moderation-bot, ticket-bot), voir
//! `sentinel_shared::event_bus::listen_stream_group` qui utilise XREADGROUP + XACK.

use std::sync::Arc;

use redis::streams::{StreamReadOptions, StreamReadReply};
use redis::AsyncCommands;
use tracing::{error, info, warn};

use crate::broadcaster::{EventBroadcaster, WsEvent};
use crate::logger::GatewayLogger;

const PAYLOAD_FIELD: &str = "payload";
const BLOCK_MS: u64 = 5_000;
const BATCH_COUNT: usize = 64;

/// Lance le tail Redis Streams avec reconnexion automatique et exponential backoff.
pub async fn run_redis_subscriber(
    redis_url: &str,
    stream_key: &str,
    broadcaster: Arc<EventBroadcaster>,
    logger: Arc<GatewayLogger>,
    base_delay_secs: u64,
    max_delay_secs: u64,
) {
    let mut delay = base_delay_secs;

    loop {
        match tail_loop(redis_url, stream_key, &broadcaster, &logger).await {
            Ok(()) => {
                warn!("Redis stream tail disconnected, reconnecting in {delay}s...");
                logger.warn(
                    "Redis stream tail deconnecte, reconnexion...",
                    serde_json::json!({
                        "event": "redis_disconnected",
                        "retry_delay_secs": delay,
                    }),
                );
            }
            Err(e) => {
                error!(error = %e, delay_secs = delay, "Redis stream tail error, reconnecting...");
                logger.error(
                    "Erreur Redis stream tail",
                    serde_json::json!({
                        "event": "redis_error",
                        "error": e.to_string(),
                        "retry_delay_secs": delay,
                    }),
                );
            }
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(delay)).await;

        // Exponential backoff: double le delay a chaque echec, jusqu'au max
        delay = (delay * 2).min(max_delay_secs);
    }
}

async fn tail_loop(
    redis_url: &str,
    stream_key: &str,
    broadcaster: &EventBroadcaster,
    logger: &GatewayLogger,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = redis::Client::open(redis_url)?;
    let mut conn = client.get_multiplexed_async_connection().await?;

    info!(stream = %stream_key, "Redis stream tail connecte");
    logger.info(
        "Redis stream tail connecte",
        serde_json::json!({
            "event": "redis_connected",
            "stream": stream_key,
        }),
    );

    // Demarrer au dernier ID : on ignore tout ce qui s'est accumule avant la connexion.
    // Cela preserve la semantique fire-and-forget de l'ancien pub/sub.
    let mut last_id = String::from("$");

    let opts = StreamReadOptions::default()
        .block(BLOCK_MS as usize)
        .count(BATCH_COUNT);

    loop {
        let reply: Option<StreamReadReply> = conn
            .xread_options(&[stream_key], &[last_id.as_str()], &opts)
            .await?;

        let Some(reply) = reply else { continue };

        for key in reply.keys {
            for entry in key.ids {
                // Extraire le champ `payload` qui contient le JSON de l'event
                let payload_str = match entry.map.get(PAYLOAD_FIELD) {
                    Some(redis::Value::BulkString(bytes)) => {
                        String::from_utf8_lossy(bytes).into_owned()
                    }
                    Some(redis::Value::SimpleString(s)) => s.clone(),
                    _ => {
                        warn!(entry_id = %entry.id, "Entry sans champ payload, ignoree");
                        last_id = entry.id.clone();
                        continue;
                    }
                };

                match serde_json::from_str::<WsEvent>(&payload_str) {
                    Ok(event) => {
                        broadcaster.broadcast(event);
                    }
                    Err(e) => {
                        warn!(error = %e, "Event stream invalide, ignore");
                    }
                }

                last_id = entry.id.clone();
            }
        }
    }
}
