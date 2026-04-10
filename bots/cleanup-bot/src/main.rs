// Phase 1 — Quick wins : jemalloc en allocateur global (Linux/macOS).
// Sur Windows MSVC, on retombe sur l'allocateur système.
#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

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

use crate::config::Config;
use crate::handler::Handler;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let config = Config::from_env("CLEANUP_DISCORD_TOKEN");

    info!(api_url = %config.api_base_url(), "Demarrage du cleanup bot");

    let intents = GatewayIntents::GUILDS
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    let base_api = Arc::new(BaseApiClient::new(&config, "cleanup-bot"));

    let mut client = Client::builder(config.discord_token(), intents)
        .event_handler(Handler)
        .cache_settings(sentinel_shared::cache_settings::minimal())
        .await
        .expect("Erreur creation du client Discord");

    {
        let mut data = client.data.write().await;
        data.insert::<ApiClientKey>(Arc::clone(&base_api));
    }

    spawn_heartbeat(Arc::clone(&base_api));

    if let Err(e) = sentinel_shared::shard_launcher::start_bot(&mut client).await {
        eprintln!("Erreur fatale : {e}");
    }
}
