mod api_client;
mod commands;
mod config;
mod handler;
mod tracker;

use serenity::prelude::*;
use tracing::info;

use crate::api_client::ApiClient;
use crate::config::Config;
use crate::handler::{ApiClientKey, Handler, TrackerKey};
use crate::tracker::StatsTracker;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let config = Config::from_env();

    info!(api_url = %config.api_base_url, "Démarrage du stats bot");

    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILD_VOICE_STATES
        | GatewayIntents::GUILDS;

    let mut client = Client::builder(&config.discord_token, intents)
        .event_handler(Handler)
        .await
        .expect("Erreur création du client Discord");

    {
        let mut data = client.data.write().await;
        data.insert::<ApiClientKey>(ApiClient::new(&config));
        data.insert::<TrackerKey>(StatsTracker::new());
    }

    // Heartbeat task
    let api_for_heartbeat = ApiClient::new(&config);
    tokio::spawn(async move {
        loop {
            if let Err(e) = api_for_heartbeat.heartbeat("stats-bot").await {
                tracing::warn!("Heartbeat failed: {}", e);
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
        }
    });

    if let Err(e) = client.start().await {
        eprintln!("Erreur fatale : {e}");
    }
}
