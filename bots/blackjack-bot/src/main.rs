// Phase 1 — Quick wins : jemalloc en allocateur global (Linux/macOS).
// Sur Windows MSVC, on retombe sur l'allocateur système.
#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

mod api_client;
mod channel_manager;
mod commands;
mod config;
mod handler;

use std::sync::Arc;

use serenity::prelude::*;
use tracing::info;

use sentinel_shared::api_client::BaseApiClient;
use sentinel_shared::config::BotConfig;
use sentinel_shared::grpc_client::{GrpcClientKey, SentinelGrpcClient};
use sentinel_shared::heartbeat::{ApiClientKey, spawn_heartbeat};

use crate::api_client::ApiClient;
use crate::channel_manager::ChannelManager;
use crate::config::Config;
use crate::handler::{ChannelManagerKey, Handler};

/// TypeMap key for the blackjack ApiClient.
pub struct GameApiKey;
impl TypeMapKey for GameApiKey {
    type Value = ApiClient;
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let config = Config::from_env("BLACKJACK_DISCORD_TOKEN");

    info!(api_url = %config.api_base_url(), "Demarrage du blackjack bot");

    let intents = GatewayIntents::GUILDS;

    let base_api = Arc::new(BaseApiClient::new(&config, "blackjack-bot"));

    // Phase 7A — gRPC interne (BlackjackService).
    let grpc = match SentinelGrpcClient::from_env().await {
        Ok(c) => Arc::new(c),
        Err(e) => panic!("SentinelGrpcClient: {e}"),
    };

    let api_client = ApiClient::new(Arc::clone(&base_api), Arc::clone(&grpc));

    let mut client = Client::builder(config.discord_token(), intents)
        .event_handler(Handler)
        .cache_settings(sentinel_shared::cache_settings::minimal())
        .await
        .expect("Erreur creation du client Discord");

    {
        let mut data = client.data.write().await;
        data.insert::<ApiClientKey>(Arc::clone(&base_api));
        data.insert::<GrpcClientKey>(Arc::clone(&grpc));
        data.insert::<GameApiKey>(api_client);
        data.insert::<ChannelManagerKey>(Arc::new(ChannelManager::new()));
    }

    spawn_heartbeat(Arc::clone(&base_api));

    if let Err(e) = sentinel_shared::shard_launcher::start_bot(&mut client).await {
        eprintln!("Erreur fatale : {e}");
    }
}
