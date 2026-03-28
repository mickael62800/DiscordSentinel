use std::collections::HashSet;

use redis::AsyncCommands;
use tracing::{error, info, warn};

use crate::config::MonitorConfig;

/// Demarre la boucle de monitoring : toutes les X secondes, verifie
/// quels bots/workers sont en ligne et alerte via l'API quand un service disparait.
pub fn start(redis_client: redis::Client, config: MonitorConfig) {
    tokio::spawn(async move {
        let interval = tokio::time::Duration::from_secs(config.check_interval_secs);
        let http = reqwest::Client::new();
        let mut previous_online: HashSet<String> = HashSet::new();
        let mut first_run = true;

        loop {
            tokio::time::sleep(interval).await;

            let mut conn = match redis_client.get_multiplexed_async_connection().await {
                Ok(c) => c,
                Err(e) => {
                    error!(error = %e, "Redis indisponible pour monitoring");
                    continue;
                }
            };

            // Recuperer tous les services connus
            let known: Vec<String> = conn.smembers("bots:known").await.unwrap_or_default();

            let mut current_online: HashSet<String> = HashSet::new();

            for name in &known {
                let exists: bool = conn
                    .exists(format!("bot:online:{}", name))
                    .await
                    .unwrap_or(false);
                if exists {
                    current_online.insert(name.clone());
                }
            }

            if first_run {
                // Premier check : on enregistre l'etat sans alerter
                previous_online = current_online;
                first_run = false;
                info!(online = previous_online.len(), total = known.len(), "Etat initial des services");
                continue;
            }

            // Detecter les services qui viennent de passer offline
            for name in &previous_online {
                if !current_online.contains(name) {
                    let is_worker = name.contains("worker");
                    let label = if is_worker { "Worker" } else { "Bot" };

                    warn!("{} hors ligne : {}", label, name);

                    // Envoyer un log a l'API
                    let _ = http
                        .post(format!("{}/api/logs", config.api_url))
                        .json(&serde_json::json!({
                            "level": "error",
                            "bot": "monitoring-worker",
                            "server": "",
                            "message": format!("{} hors ligne : {}", label, name),
                            "category": "worker",
                        }))
                        .send()
                        .await;

                    // Envoyer un evenement WebSocket via Redis pub/sub
                    let event = serde_json::json!({
                        "event": "bot_status",
                        "data": {
                            "bot": name,
                            "online": false,
                            "type": if is_worker { "worker" } else { "bot" },
                        }
                    });
                    let _: Result<(), _> = conn
                        .publish("sentinel:events", event.to_string())
                        .await;
                }
            }

            // Detecter les services qui viennent de revenir en ligne
            for name in &current_online {
                if !previous_online.contains(name) {
                    let is_worker = name.contains("worker");
                    let label = if is_worker { "Worker" } else { "Bot" };

                    info!("{} en ligne : {}", label, name);

                    let _ = http
                        .post(format!("{}/api/logs", config.api_url))
                        .json(&serde_json::json!({
                            "level": "info",
                            "bot": "monitoring-worker",
                            "server": "",
                            "message": format!("{} en ligne : {}", label, name),
                            "category": "worker",
                        }))
                        .send()
                        .await;

                    let event = serde_json::json!({
                        "event": "bot_status",
                        "data": {
                            "bot": name,
                            "online": true,
                            "type": if is_worker { "worker" } else { "bot" },
                        }
                    });
                    let _: Result<(), _> = conn
                        .publish("sentinel:events", event.to_string())
                        .await;
                }
            }

            previous_online = current_online;
        }
    });
}
