// Phase 1 — Quick wins : jemalloc en allocateur global (Linux/macOS).
// Sur Windows MSVC, on retombe sur l'allocateur système.
#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

mod api_client;
mod handler;
mod template;

use std::sync::Arc;

use serenity::prelude::*;
use tracing::info;

use sentinel_shared::api_client::BaseApiClient;
use sentinel_shared::config::{BotConfig, SimpleConfig};
use sentinel_shared::grpc_client::{GrpcClientKey, SentinelGrpcClient};
use sentinel_shared::heartbeat::{ApiClientKey, spawn_heartbeat};

use crate::handler::Handler;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "welcome_bot=info".into()),
        )
        .init();

    let config = SimpleConfig::from_env("WELCOME_DISCORD_TOKEN");

    info!(api_url = %config.api_base_url(), "Demarrage du welcome bot");

    let intents = GatewayIntents::GUILDS
        | GatewayIntents::GUILD_MEMBERS;

    let base_api = Arc::new(BaseApiClient::new(&config, "welcome-bot"));

    // Phase 7A — gRPC interne (MembersService).
    let grpc = match SentinelGrpcClient::from_env().await {
        Ok(c) => Arc::new(c),
        Err(e) => panic!("SentinelGrpcClient: {e}"),
    };

    let mut client = Client::builder(config.discord_token(), intents)
        .event_handler(Handler)
        .cache_settings(sentinel_shared::cache_settings::minimal())
        .await
        .expect("Erreur creation client Discord");

    {
        let mut data = client.data.write().await;
        data.insert::<ApiClientKey>(base_api.clone());
        data.insert::<GrpcClientKey>(grpc.clone());
    }

    spawn_heartbeat(base_api.clone());

    info!("Demarrage welcome-bot...");

    if let Err(e) = sentinel_shared::shard_launcher::start_bot(&mut client).await {
        tracing::error!(error = %e, "Erreur client Discord");
    }
}
