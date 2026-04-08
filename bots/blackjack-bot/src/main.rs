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
use crate::handler::Handler;

/// TypeMap key for the blackjack ApiClient.
pub struct GameApiKey;
impl TypeMapKey for GameApiKey {
    type Value = ApiClient;
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let config = Config::from_env("BLACKJACK_DISCORD_TOKEN");

    info!(api_url = %config.api_base_url(), "Demarrage du blackjack bot");

    let intents = GatewayIntents::GUILDS;

    let base_api = Arc::new(BaseApiClient::new(&config, "blackjack-bot"));

    let api_client = ApiClient::new(Arc::clone(&base_api));

    let mut client = Client::builder(config.discord_token(), intents)
        .event_handler(Handler)
        .await
        .expect("Erreur creation du client Discord");

    {
        let mut data = client.data.write().await;
        data.insert::<ApiClientKey>(Arc::clone(&base_api));
        data.insert::<GameApiKey>(api_client);
    }

    spawn_heartbeat(Arc::clone(&base_api));

    if let Err(e) = client.start().await {
        eprintln!("Erreur fatale : {e}");
    }
}
