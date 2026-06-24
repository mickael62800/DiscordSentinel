use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use reqwest::{Client, RequestBuilder};
use serde::Serialize;
use tokio::sync::Mutex;

use super::config::BotConfig;

/// Publisher Redis pour les events temps reel.
/// Phase 5B : XADD sur la stream `sentinel:events` (au lieu de PUBLISH pub/sub).
/// Les consumers durables (moderation-bot, ticket-bot) consomment via XREADGROUP ;
/// le Gateway lit en live-tail pour le relay WebSocket desktop.
pub struct EventPublisher {
    client: Mutex<Option<redis::aio::MultiplexedConnection>>,
    redis_url: String,
}

impl EventPublisher {
    pub fn new(redis_url: &str) -> Self {
        Self {
            client: Mutex::new(None),
            redis_url: redis_url.to_string(),
        }
    }

    /// Publie un event sur la stream (lazy-connect au premier appel).
    pub async fn publish(&self, event: &str, data: serde_json::Value) {
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
            if let Err(e) = super::event_bus::publish(conn, event, data).await {
                tracing::warn!(error = %e, "Redis XADD failed");
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
        // Initialiser le publisher Redis si REDIS_URL est defini (Phase 5B : XADD stream)
        let publisher = std::env::var("REDIS_URL")
            .ok()
            .map(|url| Arc::new(EventPublisher::new(&url)));

        // Phase 1 — Quick wins : pool keep-alive tuné pour les bots qui font
        // beaucoup d'aller-retours vers l'API interne. Le `Client` reqwest est
        // déjà un singleton (créé une seule fois par bot, partagé via la TypeMap),
        // mais les paramètres par défaut du pool ne sont pas optimaux pour notre
        // cas d'usage :
        //
        // - `pool_max_idle_per_host` (défaut 32) : on monte à 64 pour absorber
        //   les bursts (commandes Discord parallèles → multiples appels API
        //   simultanés vers le même host).
        // - `pool_idle_timeout` (défaut 90s) : on garde 5 minutes pour éviter
        //   de re-handshaker TLS toutes les 90 secondes en idle.
        // - `tcp_keepalive` : envoie un probe TCP toutes les 60s pour détecter
        //   les connexions zombies (NAT idle timeout, etc.) et les recycler.
        // - `http2_prior_knowledge` : on n'active PAS HTTP/2 par défaut car
        //   l'API expose HTTP/1.1 — laisser reqwest négocier ALPN si TLS.
        //
        // Gain typique : -50 à -80 % de latence sur les appels API internes
        // (élimine le TCP handshake + TLS handshake sur chaque requête).
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .pool_max_idle_per_host(64)
            .pool_idle_timeout(Duration::from_secs(300))
            .tcp_keepalive(Some(Duration::from_secs(60)))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            client,
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
        owner_id: Option<&str>,
    ) -> Result<(), String> {
        #[derive(Serialize)]
        struct Payload {
            guild_id: String,
            name: String,
            member_count: Option<i32>,
            #[serde(skip_serializing_if = "Option::is_none")]
            owner_id: Option<String>,
        }

        let req = self
            .client
            .post(format!("{}/api/guilds/register", self.base_url))
            .json(&Payload {
                guild_id: guild_id.to_string(),
                name: name.to_string(),
                member_count: Some(member_count),
                owner_id: owner_id.map(String::from),
            });

        self.auth(req)
            .send()
            .await
            .map_err(|e| format!("Guild register failed: {e}"))?;

        Ok(())
    }

    /// DELETE /api/guilds/{guild_id} — signale que le bot a ete retire d'un
    /// serveur (event `guild_delete`). Le selecteur web cesse de l'afficher.
    pub async fn delete_guild(&self, guild_id: &str) -> Result<(), String> {
        let req = self
            .client
            .delete(format!("{}/api/guilds/{}", self.base_url, guild_id));

        self.auth(req)
            .send()
            .await
            .map_err(|e| format!("Guild delete failed: {e}"))?;

        Ok(())
    }

    /// POST /api/guilds/reconcile — envoie la liste complete des serveurs dont
    /// le bot fait partie (au demarrage). L'API supprime les serveurs absents
    /// (retraits survenus pendant que le bot etait hors ligne).
    pub async fn reconcile_guilds(&self, guild_ids: &[String]) -> Result<(), String> {
        #[derive(Serialize)]
        struct Payload<'a> {
            guild_ids: &'a [String],
        }

        let req = self
            .client
            .post(format!("{}/api/guilds/reconcile", self.base_url))
            .json(&Payload { guild_ids });

        self.auth(req)
            .send()
            .await
            .map_err(|e| format!("Guild reconcile failed: {e}"))?;

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

    /// Comme send_bot_log mais avec un payload JSON structurel
    /// (event_type, command, user_id, etc.). Permet le filtrage cote
    /// frontend par type de commande / module.
    pub fn send_bot_log_with_details(
        &self,
        level: &str,
        message: &str,
        details: serde_json::Value,
    ) {
        #[derive(Serialize)]
        struct LogPayloadWithDetails {
            level: String,
            bot: String,
            server: String,
            message: String,
            category: String,
            details: serde_json::Value,
        }

        let log_data = LogPayloadWithDetails {
            level: level.to_string(),
            bot: self.bot_name.clone(),
            server: String::new(),
            message: message.to_string(),
            category: "bot".to_string(),
            details: details.clone(),
        };

        // Publier aussi via Redis pour le temps reel desktop
        self.publish_event("bot_log", serde_json::json!({
            "level": log_data.level,
            "bot": log_data.bot,
            "server": log_data.server,
            "message": log_data.message,
            "category": log_data.category,
            "details": details,
        }));

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

    /// Variante qui permet de specifier un `bot_name` arbitraire (utile pour
    /// le binaire unifie `sentinel-bot` qui doit lire la config des sous-modules
    /// `coude-bot`, `automod-bot`, etc., stockee sous leur nom d'origine en DB).
    pub async fn get_guild_config_for(
        &self,
        guild_id: &str,
        bot_name: &str,
    ) -> Result<HashMap<String, String>, String> {
        let url = format!(
            "{}/api/bots/config/{}/{}",
            self.base_url, guild_id, bot_name
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
            .map_err(|e| network_error_message("GET", path, &e.to_string()))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(friendly_api_error("GET", path, status, &body));
        }
        resp.json::<T>()
            .await
            .map_err(|e| parse_error_message("GET", path, &e.to_string()))
    }

    /// POST JSON vers l'API. Retourne le body deserialise.
    pub async fn post_json<B: serde::Serialize, T: serde::de::DeserializeOwned>(&self, path: &str, body: &B) -> Result<T, String> {
        let req = self.client.post(format!("{}{}", self.base_url, path)).json(body);
        let resp = self.auth(req)
            .send()
            .await
            .map_err(|e| network_error_message("POST", path, &e.to_string()))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(friendly_api_error("POST", path, status, &text));
        }
        resp.json::<T>()
            .await
            .map_err(|e| parse_error_message("POST", path, &e.to_string()))
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
            .map_err(|e| network_error_message("DELETE", path, &e.to_string()))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(friendly_api_error("DELETE", path, status, &body));
        }
        resp.json::<T>()
            .await
            .map_err(|e| parse_error_message("DELETE", path, &e.to_string()))
    }

    /// DELETE JSON avec body vers l'API. Retourne le body deserialise.
    pub async fn delete_with_body<B: serde::Serialize, T: serde::de::DeserializeOwned>(&self, path: &str, body: &B) -> Result<T, String> {
        let req = self.client.delete(format!("{}{}", self.base_url, path)).json(body);
        let resp = self.auth(req)
            .send()
            .await
            .map_err(|e| network_error_message("DELETE", path, &e.to_string()))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(friendly_api_error("DELETE", path, status, &text));
        }
        resp.json::<T>()
            .await
            .map_err(|e| parse_error_message("DELETE", path, &e.to_string()))
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

// ── Erreurs user-friendly ───────────────────────────────────────────────
//
// Les erreurs API exposees aux utilisateurs Discord doivent etre lisibles
// (sans status code, methode HTTP ou path). Ces helpers extraient le
// message metier du body JSON quand c'est une 4xx, et renvoient un
// message generique sinon. La version technique est loggee via tracing
// pour le debug.

/// Construit un message d'erreur lisible a partir d'une reponse HTTP non-2xx.
/// - 4xx : tente de parser `{"error": "..."}` et retourne juste le message.
/// - 5xx : retourne un message generique "service indisponible".
/// - autre : fallback generique.
fn friendly_api_error(method: &str, path: &str, status: reqwest::StatusCode, body: &str) -> String {
    tracing::warn!(
        method,
        path,
        status = %status,
        body,
        "Erreur API"
    );

    if status.is_client_error() {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(body) {
            if let Some(msg) = v.get("error").and_then(|e| e.as_str()) {
                return strip_error_prefix(msg).to_string();
            }
            if let Some(msg) = v.get("message").and_then(|e| e.as_str()) {
                return strip_error_prefix(msg).to_string();
            }
        }
        if !body.is_empty() && body.len() < 300 {
            return strip_error_prefix(body).to_string();
        }
        return "Requete refusee.".to_string();
    }

    if status.is_server_error() {
        return "Service temporairement indisponible. Reessaie dans un instant.".to_string();
    }

    "Une erreur est survenue.".to_string()
}

/// Retire les prefixes techniques courants (`Donnees invalides : `, etc.)
/// pour ne garder que le message metier.
fn strip_error_prefix(msg: &str) -> &str {
    const PREFIXES: &[&str] = &[
        "Donnees invalides : ",
        "Donnees invalides: ",
        "Données invalides : ",
        "Données invalides: ",
        "Validation : ",
        "Validation: ",
    ];
    for p in PREFIXES {
        if let Some(rest) = msg.strip_prefix(p) {
            return rest;
        }
    }
    msg
}

fn network_error_message(method: &str, path: &str, detail: &str) -> String {
    tracing::warn!(method, path, detail, "Erreur reseau API");
    "Connexion a l'API impossible. Reessaie dans un instant.".to_string()
}

fn parse_error_message(method: &str, path: &str, detail: &str) -> String {
    tracing::warn!(method, path, detail, "Erreur parsing reponse API");
    "Reponse de l'API illisible. Reessaie dans un instant.".to_string()
}
