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
use sentinel_shared::grpc_client::{GrpcClientKey, SentinelGrpcClient};
use sentinel_shared::heartbeat::{ApiClientKey, spawn_heartbeat};

use crate::api_client::ApiClient;
use crate::config::Config;
use crate::handler::{Handler, RolesApiKey};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "roles_bot=info".into()),
        )
        .init();

    let config = Config::from_env("ROLES_DISCORD_TOKEN");
    let base_api = Arc::new(BaseApiClient::new(&config, "roles-bot"));

    // Phase 7A — gRPC interne (RolePanelsService).
    let grpc = match SentinelGrpcClient::from_env().await {
        Ok(c) => Arc::new(c),
        Err(e) => panic!("SentinelGrpcClient: {e}"),
    };

    let roles_api = ApiClient::new(base_api.clone(), Arc::clone(&grpc));

    let intents = GatewayIntents::GUILDS
        | GatewayIntents::GUILD_MEMBERS
        | GatewayIntents::GUILD_MESSAGE_REACTIONS;

    let mut client = Client::builder(config.discord_token(), intents)
        .event_handler(Handler)
        .cache_settings(sentinel_shared::cache_settings::small())
        .await
        .expect("Erreur creation client Discord");

    {
        let mut data = client.data.write().await;
        data.insert::<ApiClientKey>(base_api.clone());
        data.insert::<GrpcClientKey>(Arc::clone(&grpc));
        data.insert::<RolesApiKey>(roles_api);
    }

    spawn_heartbeat(base_api);

    info!("Demarrage roles-bot...");

    if let Err(e) = sentinel_shared::shard_launcher::start_bot(&mut client).await {
        tracing::error!(error = %e, "Erreur client Discord");
    }
}
