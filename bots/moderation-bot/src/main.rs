// Phase 1 — Quick wins : jemalloc en allocateur global (Linux/macOS).
// Sur Windows MSVC, on retombe sur l'allocateur système.
#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

mod api_client;
mod commands;
mod config;
mod handler;
mod reason_templates;
mod risk_check;

use std::sync::Arc;

use dashmap::DashMap;
use serenity::prelude::*;
use tracing::info;

use sentinel_shared::api_client::BaseApiClient;
use sentinel_shared::config::BotConfig;
use sentinel_shared::heartbeat::{ApiClientKey, spawn_heartbeat};

use crate::api_client::ApiClient;
use crate::config::Config;
use crate::handler::{Handler, ModerationApiKey, PendingActionsKey};
use crate::risk_check::RiskyPendingKey;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let config = Config::from_env("MODERATION_DISCORD_TOKEN");

    info!(api_url = %config.base().api_base_url, "Demarrage du moderation bot");

    let intents = GatewayIntents::GUILDS | GatewayIntents::GUILD_MEMBERS;

    let base_api = Arc::new(BaseApiClient::new(&config, "moderation-bot"));
    let mod_api = ApiClient::new(BaseApiClient::new(&config, "moderation-bot"));

    let mut client = Client::builder(config.base().discord_token.as_str(), intents)
        .event_handler(Handler)
        .cache_settings(sentinel_shared::cache_settings::full())
        .await
        .expect("Erreur creation du client Discord");

    {
        let mut data = client.data.write().await;
        data.insert::<ApiClientKey>(Arc::clone(&base_api));
        data.insert::<ModerationApiKey>(mod_api);
        data.insert::<PendingActionsKey>(DashMap::new());
        data.insert::<RiskyPendingKey>(DashMap::new());
    }

    // Heartbeat via shared
    spawn_heartbeat(base_api);

    if let Err(e) = sentinel_shared::shard_launcher::start_bot(&mut client).await {
        eprintln!("Erreur fatale : {e}");
    }
}
