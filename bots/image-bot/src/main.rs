mod api_client;
mod config;
mod handler;

use std::sync::Arc;

use dashmap::DashSet;
use serenity::prelude::*;
use tracing::info;

use sentinel_shared::api_client::BaseApiClient;
use sentinel_shared::config::BotConfig;
use sentinel_shared::heartbeat::{ApiClientKey, spawn_heartbeat};

use crate::config::Config;
use crate::handler::{Handler, MaxImageSizeKey, ProcessedMessagesKey};

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
        api_url = %config.base().api_base_url,
        max_image_size = config.max_image_size,
        "Demarrage de l'image bot"
    );

    // Intents : lire les messages pour detecter les pieces jointes
    let intents = GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT;

    let base_api = Arc::new(BaseApiClient::new(&config, "image-bot"));

    let mut client = Client::builder(config.base().discord_token.as_str(), intents)
        .event_handler(Handler)
        .await
        .expect("Erreur creation du client Discord");

    // Stocker les donnees partagees
    {
        let mut data = client.data.write().await;
        data.insert::<ApiClientKey>(Arc::clone(&base_api));
        data.insert::<ProcessedMessagesKey>(Arc::new(DashSet::new()));
        data.insert::<MaxImageSizeKey>(config.max_image_size);
    }

    // Heartbeat task
    spawn_heartbeat(Arc::clone(&base_api));

    if let Err(e) = client.start().await {
        eprintln!("Erreur fatale : {e}");
    }
}
