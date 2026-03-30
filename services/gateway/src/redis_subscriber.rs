use std::sync::Arc;

use futures_util::StreamExt;
use tracing::{error, info, warn};

use crate::broadcaster::{EventBroadcaster, WsEvent};
use crate::logger::GatewayLogger;

/// Lance le subscriber Redis avec reconnexion automatique et exponential backoff.
pub async fn run_redis_subscriber(
    redis_url: &str,
    channel: &str,
    broadcaster: Arc<EventBroadcaster>,
    logger: Arc<GatewayLogger>,
    base_delay_secs: u64,
    max_delay_secs: u64,
) {
    let mut delay = base_delay_secs;

    loop {
        match subscribe_loop(redis_url, channel, &broadcaster, &logger).await {
            Ok(()) => {
                warn!("Redis subscriber disconnected, reconnecting in {delay}s...");
                logger.warn("Redis pub/sub deconnecte, reconnexion...", serde_json::json!({
                    "event": "redis_disconnected",
                    "retry_delay_secs": delay,
                }));
            }
            Err(e) => {
                error!(error = %e, delay_secs = delay, "Redis subscriber error, reconnecting...");
                logger.error("Erreur Redis pub/sub", serde_json::json!({
                    "event": "redis_error",
                    "error": e.to_string(),
                    "retry_delay_secs": delay,
                }));
            }
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(delay)).await;

        // Exponential backoff: double le delay a chaque echec, jusqu'au max
        delay = (delay * 2).min(max_delay_secs);
    }
}

async fn subscribe_loop(
    redis_url: &str,
    channel: &str,
    broadcaster: &EventBroadcaster,
    logger: &GatewayLogger,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = redis::Client::open(redis_url)?;
    let mut pubsub = client.get_async_pubsub().await?;

    pubsub.subscribe(channel).await?;
    info!(channel = %channel, "Redis pub/sub connecte");
    logger.info("Redis pub/sub connecte", serde_json::json!({
        "event": "redis_connected",
        "channel": channel,
    }));

    let mut stream = pubsub.on_message();

    while let Some(msg) = stream.next().await {
        // Reset backoff on successful message reception (connexion stable)
        let payload: String = match msg.get_payload() {
            Ok(p) => p,
            Err(e) => {
                warn!(error = %e, "Failed to get Redis message payload, skipping");
                continue; // Ne pas quitter la boucle pour un message malformed
            }
        };

        match serde_json::from_str::<WsEvent>(&payload) {
            Ok(event) => {
                broadcaster.broadcast(event);
            }
            Err(e) => {
                warn!(error = %e, "Event Redis invalide, ignore");
            }
        }
    }

    Ok(())
}
