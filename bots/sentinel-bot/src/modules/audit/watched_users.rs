//! Helpers de tracking des utilisateurs surveilles.
//!
//! Exposes comme fonctions libres (plus de `impl Handler`).
//!
//! Phase 6A : le refresh periodique est declenche par `audit-cache-worker`
//! qui publie sur `sentinel:events` un event `watched_users_refreshed`.
//! Le bot consomme cet event et refresh son cache local en pulling l'API
//! (`get_all_watched_user_ids`). Plus d'acces Redis direct cote bot.

use serenity::prelude::*;
use tracing::{info, warn};

use sentinel_shared::heartbeat::ApiClientKey;

use super::api_client::ApiClient;
use super::WatchedUserIdsKey;

/// Verifie si un utilisateur est dans le set des utilisateurs surveilles.
///
/// Lecture non-async depuis la TypeMap deja verrouillee par l'appelant
/// (evite un second `data.read().await` cote sub-handler).
pub fn is_watched(ctx_data: &TypeMap, user_id: &str) -> bool {
    ctx_data
        .get::<WatchedUserIdsKey>()
        .map(|set| set.contains(user_id))
        .unwrap_or(false)
}

/// Enregistre une activite d'un utilisateur surveille via l'API.
///
/// Silencieusement ignore si l'utilisateur n'est pas surveille — les
/// appelants peuvent appeler sans precaution.
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
    if !is_watched(&data, user_id) {
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

/// Bootstrap du cache `WatchedUserIdsKey` au demarrage du bot.
///
/// Le bot lit la liste cote API (qui sert depuis sa propre source de
/// verite). Plus d'acces Redis direct — l'audit-cache-worker continue a
/// pousser un cache mais c'est l'API qui le sert (transparence pour le bot).
pub async fn bootstrap_watched_users(ctx: &Context) {
    let data = ctx.data.read().await;
    let Some(watched_set) = data.get::<WatchedUserIdsKey>().cloned() else {
        warn!("bootstrap_watched_users: WatchedUserIdsKey manquant");
        return;
    };
    let api_client = data.get::<ApiClientKey>().cloned();
    drop(data);

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
            info!(count = watched_set.len(), "watched_users bootstrap via API");
        }
        Err(e) => {
            warn!(error = %e, "bootstrap_watched_users: API call failed, cache reste vide");
        }
    }
}

/// Consumer stream : recoit `watched_users_refreshed` (publie par
/// `audit-cache-worker`) et refresh le cache local en pulling l'API.
///
/// Avant : lisait directement Redis. Maintenant : delegue a l'API via
/// `get_all_watched_user_ids` — l'event est juste un trigger.
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

    let data = ctx.data.read().await;
    let Some(watched_set) = data.get::<WatchedUserIdsKey>().cloned() else {
        return;
    };
    let api_client = data.get::<ApiClientKey>().cloned();
    drop(data);

    let Some(base) = api_client else {
        warn!("handle_watched_refresh_event: ApiClientKey manquant");
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
                "watched_users cache refresh depuis event (via API)"
            );
        }
        Err(e) => {
            warn!(error = %e, "handle_watched_refresh_event: API call failed");
        }
    }
}
