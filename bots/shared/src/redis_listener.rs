/// Redis pub/sub listener generique avec reconnexion automatique.
/// Remplace les implementations dupliquees dans ticket-bot et moderation-bot.

use futures_util::StreamExt;
use tracing::{error, info, warn};

/// Lance un listener Redis avec reconnexion automatique.
/// `handler` est appele pour chaque message recu.
pub async fn listen_redis<F, Fut>(handler: F)
where
    F: Fn(String) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = ()> + Send,
{
    let redis_url = std::env::var("REDIS_URL")
        .unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    let channel = std::env::var("REDIS_CHANNEL")
        .unwrap_or_else(|_| "sentinel:events".to_string());

    loop {
        match redis::Client::open(redis_url.as_str()) {
            Ok(client) => {
                match client.get_async_pubsub().await {
                    Ok(mut pubsub) => {
                        if let Err(e) = pubsub.subscribe(&channel).await {
                            error!(error = %e, "Erreur abonnement Redis");
                            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                            continue;
                        }

                        info!(channel = %channel, "Redis pub/sub connecte");

                        loop {
                            let msg = pubsub.on_message().next().await;
                            if let Some(msg) = msg {
                                let payload: String = match msg.get_payload() {
                                    Ok(p) => p,
                                    Err(_) => continue,
                                };
                                handler(payload).await;
                            } else {
                                warn!("Connexion Redis perdue, reconnexion...");
                                break;
                            }
                        }
                    }
                    Err(e) => error!(error = %e, "Erreur connexion Redis"),
                }
            }
            Err(e) => error!(error = %e, "Erreur creation client Redis"),
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    }
}
