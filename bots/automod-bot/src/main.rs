mod adaptive_slowmode;
mod api_client;
mod commands;
mod config;
mod detectors;
mod handler;

use std::sync::Arc;

use dashmap::DashMap;
use serenity::prelude::*;
use tracing::info;

use sentinel_shared::api_client::BaseApiClient;
use sentinel_shared::config::BotConfig;
use sentinel_shared::heartbeat::{ApiClientKey, spawn_heartbeat};

use crate::config::Config;
use crate::adaptive_slowmode::SlowmodeTracker;
use crate::handler::{FloodTrackerKey, Handler, ProcessedMessagesKey, SlowmodeTrackerKey};

#[tokio::main]
async fn main() {
    // Charger .env
    dotenvy::dotenv().ok();

    // Initialiser le logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let config = Config::from_env("AUTOMOD_DISCORD_TOKEN");

    info!(api_url = %config.base().api_base_url, "Demarrage de l'automod bot");

    // Intents necessaires : lire les messages dans les guilds
    let intents = GatewayIntents::GUILDS
        | GatewayIntents::GUILD_MEMBERS
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    let base_api = Arc::new(BaseApiClient::new(&config, "automod-bot"));

    let mut client = Client::builder(config.base().discord_token.as_str(), intents)
        .event_handler(Handler)
        .await
        .expect("Erreur creation du client Discord");

    // Stocker le BaseApiClient et les structures partagees dans le contexte
    {
        let mut data = client.data.write().await;
        data.insert::<ApiClientKey>(Arc::clone(&base_api));
        data.insert::<ProcessedMessagesKey>(Arc::new(DashMap::new()));
        data.insert::<FloodTrackerKey>(Arc::new(DashMap::new()));
        data.insert::<SlowmodeTrackerKey>(SlowmodeTracker::new(30));
    }

    // Heartbeat task
    spawn_heartbeat(Arc::clone(&base_api));

    if let Err(e) = client.start().await {
        eprintln!("Erreur fatale : {e}");
    }
}
