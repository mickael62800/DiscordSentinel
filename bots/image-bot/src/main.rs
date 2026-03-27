mod api_client;
mod config;
mod handler;

use std::sync::Arc;

use dashmap::DashSet;
use serenity::prelude::*;
use tracing::info;

use crate::api_client::ApiClient;
use crate::config::Config;
use crate::handler::{ApiClientKey, Handler, ProcessedMessagesKey};

#[tokio::main]
async fn main() {
    // Charger .env
    dotenvy::dotenv().ok();

    // Initialiser le logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let config = Config::from_env();

    info!(
        api_url = %config.api_base_url,
        max_image_size = config.max_image_size,
        "Demarrage de l'image bot"
    );

    // Intents : lire les messages pour detecter les pieces jointes
    let intents = GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT;

    let mut client = Client::builder(&config.discord_token, intents)
        .event_handler(Handler)
        .await
        .expect("Erreur creation du client Discord");

    // Stocker les donnees partagees
    {
        let mut data = client.data.write().await;
        data.insert::<ApiClientKey>(ApiClient::new(&config));
        data.insert::<ProcessedMessagesKey>(Arc::new(DashSet::new()));
    }

    // Heartbeat task
    let api_for_heartbeat = ApiClient::new(&config);
    tokio::spawn(async move {
        loop {
            if let Err(e) = api_for_heartbeat.heartbeat("image-bot").await {
                tracing::warn!("Heartbeat failed: {}", e);
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
        }
    });

    if let Err(e) = client.start().await {
        eprintln!("Erreur fatale : {e}");
    }
}
