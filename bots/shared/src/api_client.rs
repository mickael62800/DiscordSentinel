use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use reqwest::{Client, RequestBuilder};
use serde::Serialize;
use tokio::sync::Mutex;

use crate::config::BotConfig;

/// Publisher Redis pour les events temps reel.
/// Publie sur `sentinel:events` — le Gateway relay vers le desktop.
pub struct EventPublisher {
    client: Mutex<Option<redis::aio::MultiplexedConnection>>,
    redis_url: String,
    channel: String,
}

impl EventPublisher {
    pub fn new(redis_url: &str, channel: &str) -> Self {
        Self {
            client: Mutex::new(None),
            redis_url: redis_url.to_string(),
            channel: channel.to_string(),
        }
    }

    /// Publie un event sur Redis (lazy-connect au premier appel).
    pub async fn publish(&self, event: &str, data: serde_json::Value) {
        let payload = serde_json::json!({
            "event": event,
            "data": data,
        });

        let payload_str = match serde_json::to_string(&payload) {
            Ok(s) => s,
            Err(_) => return,
        };

        let mut guard = self.client.lock().await;

        if guard.is_none() {
            match redis::Client::open(self.redis_url.as_str()) {
                Ok(client) => match client.get_multiplexed_async_connection().await {
                    Ok(conn) => *guard = Some(conn),
                    Err(e) => {
                        tracing::warn!(error = %e, "Redis connect failed for event publisher");
                        return;
                    }
                },
                Err(e) => {
                    tracing::warn!(error = %e, "Redis client creation failed");
                    return;
                }
            }
        }

        if let Some(ref mut conn) = *guard {
            use redis::AsyncCommands;
            if let Err(e) = conn.publish::<_, _, ()>(&self.channel, &payload_str).await {
                tracing::warn!(error = %e, "Redis publish failed");
                *guard = None;
            }
        }
    }
}

/// Client HTTP de base partage entre tous les bots.
/// Fournit : heartbeat, register_guild, send_log, get_guild_config, config helpers, event publishing.
pub struct BaseApiClient {
    client: Client,
    base_url: String,
    api_key: String,
    bot_name: String,
    event_publisher: Option<Arc<EventPublisher>>,
}

impl BaseApiClient {
    pub fn new<C: BotConfig>(config: &C, bot_name: &str) -> Self {
        // Initialiser le publisher Redis si REDIS_URL est defini
        let publisher = std::env::var("REDIS_URL").ok().map(|url| {
            let channel = std::env::var("REDIS_CHANNEL")
                .unwrap_or_else(|_| "sentinel:events".to_string());
            Arc::new(EventPublisher::new(&url, &channel))
        });

        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .unwrap_or_else(|_| Client::new()),
            base_url: config.api_base_url().to_string(),
            api_key: config.api_key().to_string(),
            bot_name: bot_name.to_string(),
            event_publisher: publisher,
        }
    }

    /// Retourne le client HTTP pour les requetes specifiques au bot.
    pub fn client(&self) -> &Client {
        &self.client
    }

    /// Retourne l'URL de base de l'API.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Retourne le nom du bot.
    pub fn bot_name(&self) -> &str {
        &self.bot_name
    }

    /// Ajoute l'authentification Bearer si une cle API est configuree.
    pub fn auth(&self, req: RequestBuilder) -> RequestBuilder {
        if self.api_key.is_empty() {
            req
        } else {
            req.bearer_auth(&self.api_key)
        }
    }

    // ── Heartbeat ──

    pub async fn heartbeat(&self) -> Result<(), String> {
        #[derive(Serialize)]
        struct Payload {
            name: String,
        }

        let req = self
            .client
            .post(format!("{}/api/bots/heartbeat", self.base_url))
            .json(&Payload {
                name: self.bot_name.clone(),
            });

        self.auth(req)
            .send()
            .await
            .map_err(|e| format!("Heartbeat failed: {e}"))?;

        Ok(())
    }

    // ── Guild Registration ──

    pub async fn register_guild(
        &self,
        guild_id: &str,
        name: &str,
        member_count: i32,
    ) -> Result<(), String> {
        #[derive(Serialize)]
        struct Payload {
            guild_id: String,
            name: String,
            member_count: Option<i32>,
        }

        let req = self
            .client
            .post(format!("{}/api/guilds/register", self.base_url))
            .json(&Payload {
                guild_id: guild_id.to_string(),
                name: name.to_string(),
                member_count: Some(member_count),
            });

        self.auth(req)
            .send()
            .await
            .map_err(|e| format!("Guild register failed: {e}"))?;

        Ok(())
    }

    // ── Event Publishing (Redis temps reel) ──

    /// Publie un event temps reel via Redis pour le Gateway → desktop app.
    /// Fire-and-forget : ne bloque pas le bot.
    pub fn publish_event(&self, event: &str, data: serde_json::Value) {
        if let Some(ref publisher) = self.event_publisher {
            let publisher = Arc::clone(publisher);
            let event = event.to_string();
            tokio::spawn(async move {
                publisher.publish(&event, data).await;
            });
        }
    }

    // ── Logging ──

    pub fn send_log(&self, level: &str, server: &str, message: &str) {
        self.send_log_with_category(level, server, message, "discord");
    }

    pub fn send_bot_log(&self, level: &str, message: &str) {
        self.send_log_with_category(level, "", message, "bot");
    }

    fn send_log_with_category(
        &self,
        level: &str,
        server: &str,
        message: &str,
        category: &str,
    ) {
        #[derive(Serialize)]
        struct LogPayload {
            level: String,
            bot: String,
            server: String,
            message: String,
            category: String,
        }

        let log_data = LogPayload {
            level: level.to_string(),
            bot: self.bot_name.clone(),
            server: server.to_string(),
            message: message.to_string(),
            category: category.to_string(),
        };

        // Publier aussi via Redis pour le temps reel desktop
        self.publish_event("bot_log", serde_json::json!({
            "level": log_data.level,
            "bot": log_data.bot,
            "server": log_data.server,
            "message": log_data.message,
            "category": log_data.category,
        }));

        // Persister via HTTP (fire-and-forget)
        let req = self
            .client
            .post(format!("{}/api/logs", self.base_url))
            .json(&log_data);

        let req = self.auth(req);
        tokio::spawn(async move {
            if let Err(e) = req.send().await {
                tracing::warn!("Log send failed: {e}");
            }
        });
    }

    // ── Guild Config ──

    pub async fn get_guild_config(
        &self,
        guild_id: &str,
    ) -> Result<HashMap<String, String>, String> {
        let url = format!(
            "{}/api/bots/config/{}/{}",
            self.base_url, guild_id, self.bot_name
        );
        let req = self.client.get(&url);

        #[derive(serde::Deserialize)]
        struct ConfigEntry {
            config_key: String,
            config_value: String,
        }

        let resp = self
            .auth(req)
            .send()
            .await
            .map_err(|e| format!("Config fetch failed: {e}"))?;

        let entries: Vec<ConfigEntry> = resp
            .json()
            .await
            .map_err(|e| format!("Config parse failed: {e}"))?;

        Ok(entries
            .into_iter()
            .map(|e| (e.config_key, e.config_value))
            .collect())
    }

    // ── HTTP Helpers ──
    // Eliminent le boilerplate repete dans chaque api_client de bot.

    /// GET JSON vers l'API. Retourne le body deserialise.
    pub async fn get_json<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T, String> {
        let req = self.client.get(format!("{}{}", self.base_url, path));
        let resp = self.auth(req)
            .send()
            .await
            .map_err(|e| format!("Erreur reseau GET {path}: {e}"))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Erreur API {status} GET {path}: {body}"));
        }
        resp.json::<T>()
            .await
            .map_err(|e| format!("Erreur parsing GET {path}: {e}"))
    }

    /// POST JSON vers l'API. Retourne le body deserialise.
    pub async fn post_json<B: serde::Serialize, T: serde::de::DeserializeOwned>(&self, path: &str, body: &B) -> Result<T, String> {
        let req = self.client.post(format!("{}{}", self.base_url, path)).json(body);
        let resp = self.auth(req)
            .send()
            .await
            .map_err(|e| format!("Erreur reseau POST {path}: {e}"))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("Erreur API {status} POST {path}: {text}"));
        }
        resp.json::<T>()
            .await
            .map_err(|e| format!("Erreur parsing POST {path}: {e}"))
    }

    /// POST fire-and-forget vers l'API. Log l'erreur mais ne la propage pas.
    pub async fn post_fire_and_forget<B: serde::Serialize>(&self, path: &str, body: &B) {
        let req = self.client.post(format!("{}{}", self.base_url, path)).json(body);
        if let Err(e) = self.auth(req).send().await {
            tracing::warn!(error = %e, path, "Echec POST fire-and-forget");
        }
    }

    /// PATCH JSON vers l'API. Fire-and-forget avec log d'erreur.
    pub async fn patch_fire_and_forget<B: serde::Serialize>(&self, path: &str, body: &B) {
        let req = self.client.patch(format!("{}{}", self.base_url, path)).json(body);
        if let Err(e) = self.auth(req).send().await {
            tracing::warn!(error = %e, path, "Echec PATCH fire-and-forget");
        }
    }

    /// DELETE JSON vers l'API. Retourne le body deserialise.
    pub async fn delete_json<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T, String> {
        let req = self.client.delete(format!("{}{}", self.base_url, path));
        let resp = self.auth(req)
            .send()
            .await
            .map_err(|e| format!("Erreur reseau DELETE {path}: {e}"))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Erreur API {status} DELETE {path}: {body}"));
        }
        resp.json::<T>()
            .await
            .map_err(|e| format!("Erreur parsing DELETE {path}: {e}"))
    }

    /// DELETE JSON avec body vers l'API. Retourne le body deserialise.
    pub async fn delete_with_body<B: serde::Serialize, T: serde::de::DeserializeOwned>(&self, path: &str, body: &B) -> Result<T, String> {
        let req = self.client.delete(format!("{}{}", self.base_url, path)).json(body);
        let resp = self.auth(req)
            .send()
            .await
            .map_err(|e| format!("Erreur reseau DELETE {path}: {e}"))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("Erreur API {status} DELETE {path}: {text}"));
        }
        resp.json::<T>()
            .await
            .map_err(|e| format!("Erreur parsing DELETE {path}: {e}"))
    }

    // ── Config Helpers ──

    pub fn config_or(
        config: &HashMap<String, String>,
        key: &str,
        default: &str,
    ) -> String {
        config
            .get(key)
            .cloned()
            .unwrap_or_else(|| default.to_string())
    }

    pub fn config_u64(
        config: &HashMap<String, String>,
        key: &str,
        default: u64,
    ) -> u64 {
        config
            .get(key)
            .and_then(|v| v.parse().ok())
            .unwrap_or(default)
    }

    pub fn config_bool(
        config: &HashMap<String, String>,
        key: &str,
        default: bool,
    ) -> bool {
        config
            .get(key)
            .map(|v| v == "true" || v == "1")
            .unwrap_or(default)
    }
}
