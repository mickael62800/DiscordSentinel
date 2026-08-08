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
    if let Some(base) = data.get::<crate::shared::grpc_client::GrpcClientKey>() {
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
    let api_client = data
        .get::<crate::shared::grpc_client::GrpcClientKey>()
        .cloned();
    drop(data);

    let Some(base) = api_client else {
        warn!("bootstrap_watched_users: GrpcClientKey manquant, cache vide");
        return;
    };
    let api = ApiClient::new(base);
    refresh_watched_for_guilds(ctx, &api, &watched_set, "bootstrap").await;
}

/// Recharge le cache des surveilles pour TOUS les serveurs du bot. Le guild_id
/// est obligatoire cote API : on interroge donc par guilde (mono-serveur = un
/// appel) et on unione les resultats avant de remplacer le cache d'un bloc.
async fn refresh_watched_for_guilds(
    ctx: &Context,
    api: &ApiClient,
    watched_set: &std::sync::Arc<dashmap::DashSet<String>>,
    origine: &str,
) {
    let mut collected: Vec<String> = Vec::new();
    let mut ok = true;
    for guild_id in ctx.cache.guilds() {
        match api.get_all_watched_user_ids(&guild_id.to_string()).await {
            Ok(ids) => collected.extend(ids),
            Err(e) => {
                ok = false;
                warn!(error = %e, guild_id = %guild_id, origine, "watched_users: refresh API echoue pour ce serveur");
            }
        }
    }
    // Ne remplace le cache que si AU MOINS un appel a reussi : sur echec total
    // (API down), on garde l'ancien cache plutot que de le vider a tort.
    if ok || !collected.is_empty() {
        watched_set.clear();
        for id in collected {
            watched_set.insert(id);
        }
        info!(
            count = watched_set.len(),
            origine, "watched_users cache rafraichi via API"
        );
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
    let api_client = data
        .get::<crate::shared::grpc_client::GrpcClientKey>()
        .cloned();
    drop(data);

    let Some(base) = api_client else {
        warn!("handle_watched_refresh_event: GrpcClientKey manquant");
        return;
    };
    let api = ApiClient::new(base);
    refresh_watched_for_guilds(ctx, &api, &watched_set, "event").await;
}
