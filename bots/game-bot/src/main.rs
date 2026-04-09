mod api_client;
mod commands;
mod detector;
mod handler;

use std::sync::Arc;

use serenity::prelude::*;
use tracing::info;

use sentinel_shared::api_client::BaseApiClient;
use sentinel_shared::config::{BotConfig, SimpleConfig};
use sentinel_shared::heartbeat::{ApiClientKey, spawn_heartbeat};

use crate::handler::Handler;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "game_bot=info".into()),
        )
        .init();

    let config = SimpleConfig::from_env("GAME_DISCORD_TOKEN");

    info!(api_url = %config.api_base_url(), "Demarrage du game bot");

    let intents = GatewayIntents::GUILDS
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    let base_api = Arc::new(BaseApiClient::new(&config, "game-bot"));

    let mut client = Client::builder(config.discord_token(), intents)
        .event_handler(Handler)
        .await
        .expect("Erreur creation client Discord");

    {
        let mut data = client.data.write().await;
        data.insert::<ApiClientKey>(base_api.clone());
    }

    spawn_heartbeat(base_api.clone());

    info!("Demarrage game-bot...");

    if let Err(e) = client.start().await {
        tracing::error!(error = %e, "Erreur client Discord");
    }
}
