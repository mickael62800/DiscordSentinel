mod api_client;
mod config;
mod detectors;
mod handler;

use serenity::prelude::*;
use tracing::info;

use crate::api_client::ApiClient;
use crate::config::Config;
use crate::handler::{ApiClientKey, Handler};

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
    }

    if let Err(e) = client.start().await {
        eprintln!("Erreur fatale : {e}");
    }
}
