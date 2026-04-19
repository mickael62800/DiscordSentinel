use std::sync::Arc;

use serenity::prelude::*;

use crate::api_client::BaseApiClient;

/// Cle TypeMap pour stocker le BaseApiClient dans le data store de Serenity.
pub struct ApiClientKey;

impl TypeMapKey for ApiClientKey {
    type Value = Arc<BaseApiClient>;
}

/// Lance une tache heartbeat en arriere-plan qui ping le backend toutes les 30 secondes.
pub fn spawn_heartbeat(api: Arc<BaseApiClient>) {
    tokio::spawn(async move {
        loop {
            if let Err(e) = api.heartbeat().await {
                tracing::warn!(bot = api.bot_name(), error = %e, "Heartbeat failed");
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
        }
    });
}

/// Helper pour enregistrer les guilds au demarrage dans le handler `ready`.
pub async fn register_guilds(
    ctx: &Context,
    ready: &serenity::model::gateway::Ready,
) {
    let data = ctx.data.read().await;
    let Some(api) = data.get::<ApiClientKey>() else {
        return;
    };

    api.send_bot_log("info", &format!("{} demarre", api.bot_name()));

    for guild_status in &ready.guilds {
        let guild_id = guild_status.id;
        if let Ok(guild) = guild_id.to_partial_guild(&ctx.http).await {
            let member_count = guild.approximate_member_count.unwrap_or(0) as i32;
            let owner_id = guild.owner_id.to_string();
            if let Err(e) = api
                .register_guild(&guild_id.to_string(), &guild.name, member_count, Some(&owner_id))
                .await
            {
                tracing::warn!(error = %e, guild = %guild.name, "Erreur enregistrement guild");
            } else {
                tracing::info!(guild = %guild.name, "Guild enregistree");
            }
        }
    }
}
