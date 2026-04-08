use serde::Serialize;
use tracing::warn;

#[derive(Debug, Clone, Serialize)]
pub struct WsEvent {
    pub event: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guild_id: Option<String>,
    pub data: serde_json::Value,
}

/// Broadcaster d'evenements — publie sur Redis pour le gateway WebSocket dedie.
pub struct EventBroadcaster {
    redis_client: Option<redis::Client>,
    redis_channel: String,
}

impl EventBroadcaster {
    pub fn new() -> Self {
        Self {
            redis_client: None,
            redis_channel: "sentinel:events".to_string(),
        }
    }

    /// Configure la publication Redis.
    pub fn with_redis(mut self, client: redis::Client, channel: String) -> Self {
        self.redis_client = Some(client);
        self.redis_channel = channel;
        self
    }

    /// Publie un evenement sur Redis pour le gateway.
    /// Le `guild_id` est extrait automatiquement du payload JSON pour le filtrage server-side.
    pub fn broadcast(&self, event: &str, data: serde_json::Value) {
        let guild_id = data.get("guild_id").and_then(|v| v.as_str()).map(String::from);

        let ws_event = WsEvent {
            event: event.to_string(),
            guild_id,
            data,
        };

        if let Some(ref client) = self.redis_client {
            let client = client.clone();
            let channel = self.redis_channel.clone();
            let json = match serde_json::to_string(&ws_event) {
                Ok(j) => j,
                Err(e) => {
                    warn!(error = %e, event = %ws_event.event, "Echec serialisation event broadcast — event perdu");
                    return;
                }
            };
            tokio::spawn(async move {
                match client.get_multiplexed_async_connection().await {
                    Ok(mut conn) => {
                        if let Err(e) = redis::AsyncCommands::publish::<_, _, ()>(&mut conn, &channel, &json).await {
                            warn!(error = %e, "Echec Redis publish event broadcast");
                        }
                    }
                    Err(e) => {
                        warn!(error = %e, "Echec connexion Redis pour broadcast");
                    }
                }
            });
        }
    }
}
