use std::sync::Arc;

use futures_util::StreamExt;
use tracing::{error, info, warn};

use crate::broadcaster::{EventBroadcaster, WsEvent};
use crate::logger::GatewayLogger;

pub async fn run_redis_subscriber(
    redis_url: &str,
    channel: &str,
    broadcaster: Arc<EventBroadcaster>,
    logger: Arc<GatewayLogger>,
) {
    loop {
        match subscribe_loop(redis_url, channel, &broadcaster, &logger).await {
            Ok(()) => {
                warn!("Redis subscriber disconnected, reconnecting in 2s...");
                logger.warn("Redis pub/sub deconnecte, reconnexion...", serde_json::json!({
                    "event": "redis_disconnected",
                }));
            }
            Err(e) => {
                error!(error = %e, "Redis subscriber error, reconnecting in 2s...");
                logger.error("Erreur Redis pub/sub", serde_json::json!({
                    "event": "redis_error",
                    "error": e.to_string(),
                }));
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
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
        let payload: String = msg.get_payload()?;
        match serde_json::from_str::<WsEvent>(&payload) {
            Ok(event) => {
                broadcaster.broadcast(event);
            }
            Err(e) => {
                warn!(error = %e, "Event Redis invalide, ignore");
                logger.warn("Event Redis invalide", serde_json::json!({
                    "event": "invalid_redis_event",
                    "error": e.to_string(),
                }));
            }
        }
    }

    Ok(())
}
