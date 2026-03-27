mod api_client;
mod config;
mod detectors;
mod handler;

use std::sync::Arc;

use dashmap::{DashMap, DashSet};
use serenity::prelude::*;
use tracing::info;

use crate::api_client::ApiClient;
use crate::config::Config;
use crate::handler::{ApiClientKey, FloodTrackerKey, Handler, ProcessedMessagesKey};

#[tokio::main]
async fn main() {
    // Charger .env
    dotenvy::dotenv().ok();

    // Initialiser le logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let config = Config::from_env();

    info!(api_url = %config.api_base_url, "Démarrage de l'automod bot");

    // Intents nécessaires : lire les messages dans les guilds
    let intents = GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT;

    let mut client = Client::builder(&config.discord_token, intents)
        .event_handler(Handler)
        .await
        .expect("Erreur création du client Discord");

    // Stocker l'ApiClient dans le contexte partagé
    {
        let mut data = client.data.write().await;
        data.insert::<ApiClientKey>(ApiClient::new(&config));
        data.insert::<ProcessedMessagesKey>(Arc::new(DashSet::new()));
        data.insert::<FloodTrackerKey>(Arc::new(DashMap::new()));
    }

    // Heartbeat task
    let api_for_heartbeat = ApiClient::new(&config);
    tokio::spawn(async move {
        loop {
            if let Err(e) = api_for_heartbeat.heartbeat("automod-bot").await {
                tracing::warn!("Heartbeat failed: {}", e);
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
        }
    });

    if let Err(e) = client.start().await {
        eprintln!("Erreur fatale : {e}");
    }
}
