mod api_client;
mod commands;
mod config;
mod handler;

use std::sync::Arc;

use serenity::prelude::*;
use tracing::info;

use sentinel_shared::api_client::BaseApiClient;
use sentinel_shared::config::BotConfig;
use sentinel_shared::heartbeat::{ApiClientKey, spawn_heartbeat};

use crate::api_client::ApiClient;
use crate::config::Config;
use crate::handler::{Handler, RolesApiKey};

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
    }

    spawn_heartbeat(base_api);

    info!("Demarrage community-bot...");

    if let Err(e) = client.start().await {
        tracing::error!(error = %e, "Erreur client Discord");
    }
}
