//! Helpers de tracking des utilisateurs surveillés.
//!
//! Exposés comme méthodes associées sur `Handler` pour préserver l'API
//! historique (`Handler::is_watched`, `Handler::track_activity`) utilisée
//! par tous les sous-handlers.

use serenity::prelude::*;
use tracing::warn;

use sentinel_shared::heartbeat::ApiClientKey;

use super::type_keys::WatchedUserIdsKey;
use super::Handler;
use crate::api_client::ApiClient;

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
