//! Helpers de tracking des utilisateurs surveillés.
//!
//! Exposés comme méthodes associées sur `Handler` pour préserver l'API
//! historique (`Handler::is_watched`, `Handler::track_activity`) utilisée
//! par tous les sous-handlers.
//!
//! Phase 6A : le refresh périodique est délégué à `audit-cache-worker` qui
//! push vers Redis + publie sur `sentinel:events`. Ce module expose les
//! helpers `bootstrap_watched_users` (startup) et `handle_watched_refresh_event`
//! (consumer stream) utilisés par `handler::ready`.

use redis::AsyncCommands;
use serenity::prelude::*;
use tracing::{info, warn};

use sentinel_shared::heartbeat::ApiClientKey;

use super::type_keys::WatchedUserIdsKey;
use super::Handler;
use super::api_client::ApiClient;

const REDIS_KEY: &str = "audit:watched_users";

impl Handler {
    /// Vérifie si un utilisateur est dans le set des utilisateurs surveillés.
    ///
    /// Lecture non-async depuis la TypeMap déjà verrouillée par l'appelant
    /// (évite un second `data.read().await` côté sub-handler).
    pub fn is_watched(ctx_data: &TypeMap, user_id: &str) -> bool {
        ctx_data
            .get::<WatchedUserIdsKey>()
            .map(|set| set.contains(user_id))
            .unwrap_or(false)
    }

    /// Enregistre une activité d'un utilisateur surveillé via l'API.
    ///
    /// Silencieusement ignoré si l'utilisateur n'est pas surveillé — les
    /// appelants peuvent appeler sans précaution.
    pub async fn track_activity(
        ctx: &Context,
        guild_id: &str,
        user_id: &str,
        event_type: &str,
        channel_id: Option<&str>,
        channel_name: Option<&str>,
        content: Option<&str>,
        metadata: serde_json::Value,
    ) {
        let data = ctx.data.read().await;
        if !Self::is_watched(&data, user_id) {
            return;
        }
        if let Some(base) = data.get::<ApiClientKey>() {
            let api = ApiClient::new(base.clone());
            if let Err(e) = api
                .log_user_activity(
                    guild_id,
                    user_id,
                    event_type,
                    channel_id,
                    channel_name,
                    content,
                    metadata,
                )
                .await
            {
                warn!(error = %e, "Failed to log user activity");
            }
        }
    }
}

/// Phase 6A — Bootstrap du cache `WatchedUserIdsKey` au demarrage du bot.
///
/// Essaie de lire le snapshot depuis Redis (alimente par `audit-cache-worker`).
/// Si Redis est vide (worker pas encore demarre, premier deploiement), retombe
/// sur un appel API direct comme fallback une seule fois.
pub async fn bootstrap_watched_users(ctx: &Context) {
    let data = ctx.data.read().await;
    let Some(watched_set) = data.get::<WatchedUserIdsKey>().cloned() else {
        warn!("bootstrap_watched_users: WatchedUserIdsKey manquant");
        return;
    };
    let api_client = data.get::<ApiClientKey>().cloned();
    drop(data);

    // 1. Tentative Redis
    let redis_url = std::env::var("REDIS_URL").unwrap_or_default();
    if !redis_url.is_empty() {
        if let Ok(client) = redis::Client::open(redis_url) {
            if let Ok(mut conn) = client.get_multiplexed_async_connection().await {
                if let Ok(Some(json)) = conn.get::<_, Option<String>>(REDIS_KEY).await {
                    if let Ok(ids) = serde_json::from_str::<Vec<String>>(&json) {
                        watched_set.clear();
                        for id in ids {
                            watched_set.insert(id);
                        }
                        info!(
                            count = watched_set.len(),
                            "watched_users bootstrap depuis Redis"
                        );
                        return;
                    }
                }
            }
        }
    }

    // 2. Fallback API (une seule fois, le worker prendra le relais au prochain tick)
    let Some(base) = api_client else {
        warn!("bootstrap_watched_users: ApiClientKey manquant, cache vide");
        return;
    };
    let api = ApiClient::new(base);
    match api.get_all_watched_user_ids().await {
        Ok(ids) => {
            watched_set.clear();
            for id in ids {
                watched_set.insert(id);
            }
            info!(
                count = watched_set.len(),
                "watched_users bootstrap via fallback API (Redis vide)"
            );
        }
        Err(e) => {
            warn!(error = %e, "bootstrap_watched_users: fallback API failed, cache reste vide");
        }
    }
}

/// Phase 6A — Consumer stream : recoit `watched_users_refreshed` et refresh
/// le cache depuis Redis. Appele par le flow `listen_stream_group` dans
/// `handler::ready`.
pub async fn handle_watched_refresh_event(ctx: &Context, payload_json: &str) {
    // Parse pour verifier le type d'event — on ignore les autres events qui
    // passent sur la meme stream (moderation_action, etc.)
    let event: serde_json::Value = match serde_json::from_str(payload_json) {
        Ok(v) => v,
        Err(_) => return,
    };
    let event_type = event.get("event").and_then(|v| v.as_str()).unwrap_or("");
    if event_type != "watched_users_refreshed" {
        return;
    }

    // Re-read depuis Redis (le worker a deja pousse le snapshot avant d'emit
    // l'event, donc le SET est a jour)
    let data = ctx.data.read().await;
    let Some(watched_set) = data.get::<WatchedUserIdsKey>().cloned() else {
        return;
    };
    drop(data);

    let redis_url = std::env::var("REDIS_URL").unwrap_or_default();
    if redis_url.is_empty() {
        return;
    }
    let client = match redis::Client::open(redis_url) {
        Ok(c) => c,
        Err(e) => {
            warn!(error = %e, "handle_watched_refresh_event: redis client failed");
            return;
        }
    };
    let mut conn = match client.get_multiplexed_async_connection().await {
        Ok(c) => c,
        Err(e) => {
            warn!(error = %e, "handle_watched_refresh_event: redis connect failed");
            return;
        }
    };
    let json: Option<String> = match conn.get(REDIS_KEY).await {
        Ok(v) => v,
        Err(e) => {
            warn!(error = %e, "handle_watched_refresh_event: redis get failed");
            return;
        }
    };
    let Some(json) = json else {
        warn!("handle_watched_refresh_event: Redis key vide (worker deconnecte ?)");
        return;
    };
    let ids: Vec<String> = match serde_json::from_str(&json) {
        Ok(v) => v,
        Err(e) => {
            warn!(error = %e, "handle_watched_refresh_event: parse json failed");
            return;
        }
    };

    watched_set.clear();
    for id in ids {
        watched_set.insert(id);
    }
    info!(count = watched_set.len(), "watched_users cache refresh depuis event");
}
