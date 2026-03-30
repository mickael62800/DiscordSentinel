mod api_client;
mod commands;
mod config;
mod handler;
mod tracker;

use std::sync::Arc;

use serenity::prelude::*;
use tracing::info;

use sentinel_shared::api_client::BaseApiClient;
use sentinel_shared::config::BotConfig;
use sentinel_shared::heartbeat::{ApiClientKey, spawn_heartbeat};

use crate::api_client::ApiClient;
use crate::config::Config;
use crate::handler::{Handler, StatsApiKey, TrackerKey};
use crate::tracker::StatsTracker;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let config = Config::from_env();

    info!(api_url = %config.api_base_url(), "Demarrage du stats bot");

    let base = Arc::new(BaseApiClient::new(&config, "stats-bot"));

    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILD_VOICE_STATES
        | GatewayIntents::GUILDS;

    let mut client = Client::builder(config.discord_token(), intents)
        .event_handler(Handler)
        .await
        .expect("Erreur creation du client Discord");

    {
        let mut data = client.data.write().await;
        data.insert::<ApiClientKey>(Arc::clone(&base));
        data.insert::<StatsApiKey>(ApiClient::new(Arc::clone(&base)));
        data.insert::<TrackerKey>(StatsTracker::new());
    }

    // Heartbeat toutes les 30 secondes via le shared helper
    spawn_heartbeat(Arc::clone(&base));

    if let Err(e) = client.start().await {
        eprintln!("Erreur fatale : {e}");
    }
}
