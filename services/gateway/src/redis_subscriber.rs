use std::sync::Arc;

use futures_util::StreamExt;
use tracing::{error, info, warn};

use crate::broadcaster::{EventBroadcaster, WsEvent};

/// Ecoute Redis pub/sub et forward les events vers le broadcaster local.
/// Reconnecte automatiquement en cas de deconnexion.
pub async fn run_redis_subscriber(
    redis_url: &str,
    channel: &str,
    broadcaster: Arc<EventBroadcaster>,
) {
    loop {
        match subscribe_loop(redis_url, channel, &broadcaster).await {
            Ok(()) => {
                warn!("Redis subscriber disconnected, reconnecting in 2s...");
            }
            Err(e) => {
                error!(error = %e, "Redis subscriber error, reconnecting in 2s...");
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    }
}

async fn subscribe_loop(
    redis_url: &str,
    channel: &str,
    broadcaster: &EventBroadcaster,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = redis::Client::open(redis_url)?;
    let mut pubsub = client.get_async_pubsub().await?;

    pubsub.subscribe(channel).await?;
    info!(channel = %channel, "Redis pub/sub connecte");

    let mut stream = pubsub.on_message();

    while let Some(msg) = stream.next().await {
        let payload: String = msg.get_payload()?;
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
