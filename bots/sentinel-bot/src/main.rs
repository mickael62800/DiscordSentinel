// Phase 1 — Quick wins : jemalloc en allocateur global (Linux/macOS).
#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

mod config;
mod handler;
mod modules;

use std::sync::Arc;

use serenity::prelude::*;
use tracing::info;

use sentinel_shared::config::BotConfig;
use sentinel_shared::grpc_client::{GrpcClientKey, SentinelGrpcClient};
use sentinel_shared::heartbeat::{ApiClientKey, spawn_heartbeat};

use crate::config::Config;
use crate::handler::Handler;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let config = Config::from_env();
    info!(api_url = %config.base().api_base_url(), "Demarrage de Sentinel Bot (unifie)");

    let api = Arc::new(sentinel_shared::api_client::BaseApiClient::new(&config, "sentinel-bot"));

    let grpc = match SentinelGrpcClient::from_env().await {
        Ok(c) => Arc::new(c),
        Err(e) => {
            eprintln!("Erreur fatale gRPC: {e}");
            std::process::exit(1);
        }
    };

    let intents = GatewayIntents::GUILDS
        | GatewayIntents::GUILD_VOICE_STATES
        | GatewayIntents::GUILD_MEMBERS
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILD_BANS;

    let mut client = Client::builder(config.discord_token(), intents)
        .event_handler(Handler)
        .cache_settings(sentinel_shared::cache_settings::full())
        .await
        .expect("Erreur creation client Discord");

    {
        let mut data = client.data.write().await;
        data.insert::<ApiClientKey>(api.clone());
        data.insert::<GrpcClientKey>(grpc);
    }

    spawn_heartbeat(api);

    info!("Sentinel Bot pret");

    if let Err(e) = sentinel_shared::shard_launcher::start_bot(&mut client).await {
        eprintln!("Erreur fatale : {e}");
    }
}
