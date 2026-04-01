mod api_client;
mod commands;
mod config;
mod exclusive_groups;
mod handler;
mod prerequisites;
mod sponsorship;
mod temp_roles;

use std::sync::Arc;

use serenity::prelude::*;
use tracing::info;

use sentinel_shared::api_client::BaseApiClient;
use sentinel_shared::config::BotConfig;
use sentinel_shared::heartbeat::{ApiClientKey, spawn_heartbeat};

use crate::api_client::ApiClient;
use crate::config::Config;
use crate::handler::{Handler, RolesApiKey, SponsorshipKey, TempRoleKey};
use crate::sponsorship::SponsorshipTracker;
use crate::temp_roles::TempRoleTracker;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "community_bot=info".into()),
        )
        .init();

    let config = Config::from_env();
    let base_api = Arc::new(BaseApiClient::new(&config, "community-bot"));
    let roles_api = ApiClient::new(base_api.clone());

    let intents = GatewayIntents::GUILDS
        | GatewayIntents::GUILD_MEMBERS
        | GatewayIntents::GUILD_MESSAGE_REACTIONS;

    let mut client = Client::builder(config.discord_token(), intents)
        .event_handler(Handler)
        .await
        .expect("Erreur creation client Discord");

    {
        let mut data = client.data.write().await;
        data.insert::<ApiClientKey>(base_api.clone());
        data.insert::<RolesApiKey>(roles_api);
        data.insert::<TempRoleKey>(TempRoleTracker::new());
        data.insert::<SponsorshipKey>(SponsorshipTracker::new());
    }

    spawn_heartbeat(base_api.clone());

    // Background task : nettoyage roles temporaires (toutes les 60s)
    let data_for_temp = Arc::clone(&client.data);
    let http_for_temp = Arc::clone(&client.http);
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;

            let data = data_for_temp.read().await;
            let tracker = match data.get::<TempRoleKey>() {
                Some(t) => t,
                None => continue,
            };

            let expired = tracker.expired();
            for temp in &expired {
                let guild_id = serenity::model::id::GuildId::new(temp.guild_id);
                let user_id = serenity::model::id::UserId::new(temp.user_id);
                let role_id = serenity::model::id::RoleId::new(temp.role_id);

                if let Ok(member) = guild_id.member(&http_for_temp, user_id).await {
                    if member.remove_role(&http_for_temp, role_id).await.is_ok() {
                        info!(
                            guild = %temp.guild_id,
                            user = %temp.user_id,
                            role = %temp.role_id,
                            "Role temporaire expire et retire"
                        );
                    }
                }
                tracker.remove(temp.guild_id, temp.user_id, temp.role_id);

                // Supprimer dans l'API aussi
                if let Some(api) = data.get::<RolesApiKey>() {
                    api.delete_temp_role(
                        &temp.guild_id.to_string(),
                        &temp.user_id.to_string(),
                        &temp.role_id.to_string(),
                    ).await;
                }
            }
        }
    });

    info!("Demarrage community-bot...");

    if let Err(e) = client.start().await {
        tracing::error!(error = %e, "Erreur client Discord");
    }
}
