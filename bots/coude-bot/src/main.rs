mod channel_check;
mod commands;
mod config;
mod db;
mod game;
pub mod guild_config;
mod handler;

use std::sync::Arc;

use serenity::prelude::*;
use sqlx::postgres::PgPoolOptions;
use tracing::info;

use sentinel_shared::api_client::BaseApiClient;
use sentinel_shared::config::BotConfig;
use sentinel_shared::heartbeat::{ApiClientKey, spawn_heartbeat};

use crate::config::Config;
use crate::db::GameDb;
use crate::handler::{GameDbKey, Handler};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let config = Config::from_env();

    info!(api_url = %config.base().api_base_url, "Demarrage du coude bot");

    // Connexion a la base de donnees
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&config.database_url)
        .await
        .expect("Impossible de se connecter a la base de donnees");

    let game_db = GameDb::new(pool);

    let intents = GatewayIntents::GUILDS | GatewayIntents::GUILD_MEMBERS;

    let base_api = Arc::new(BaseApiClient::new(&config, "coude-bot"));

    let mut client = Client::builder(config.base().discord_token.as_str(), intents)
        .event_handler(Handler)
        .await
        .expect("Erreur creation du client Discord");

    {
        let mut data = client.data.write().await;
        data.insert::<ApiClientKey>(Arc::clone(&base_api));
        data.insert::<GameDbKey>(game_db);
    }

    // Heartbeat via shared
    spawn_heartbeat(base_api);

    if let Err(e) = client.start().await {
        eprintln!("Erreur fatale : {e}");
    }
}
