// Phase 1 — Quick wins : jemalloc en allocateur global (Linux/macOS).
// Sur Windows MSVC, on retombe sur l'allocateur système.
#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

pub mod api_client;
pub mod catalog;
mod channel_check;
mod commands;
mod config;
pub mod guild_config;
mod handler;
pub mod interaction_helper;
pub mod prison_check;
pub mod taunts_dispatch;

use std::sync::Arc;

use serenity::prelude::*;
use tracing::info;

use sentinel_shared::api_client::BaseApiClient;
use sentinel_shared::config::BotConfig;
use sentinel_shared::grpc_client::{GrpcClientKey, SentinelGrpcClient};
use sentinel_shared::heartbeat::{ApiClientKey, spawn_heartbeat};

use crate::config::Config;
use crate::handler::Handler;

/// Cle TypeMap pour le client API du jeu.
pub struct GameApiKey;
impl TypeMapKey for GameApiKey {
    type Value = api_client::ApiClient;
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let config = Config::from_env();

    info!(api_url = %config.base().api_base_url, "Demarrage du coude bot");

    let intents = GatewayIntents::GUILDS | GatewayIntents::GUILD_MEMBERS;

    let base_api = Arc::new(BaseApiClient::new(&config, "coude-bot"));

    // Phase 7A — gRPC interne (CoudePlayerService).
    let grpc = match SentinelGrpcClient::from_env().await {
        Ok(c) => Arc::new(c),
        Err(e) => {
            eprintln!("Erreur fatale: impossible d'initialiser SentinelGrpcClient: {e}");
            std::process::exit(1);
        }
    };

    let api_client = api_client::ApiClient::new(Arc::clone(&base_api), Arc::clone(&grpc));

    // Phase 8 : fetch du catalogue Coude (classes, shop, progression) depuis
    // l'API. Le bot ne contient plus aucune donnee metier en dur — tout vient
    // de l'API via ce RPC au boot. Si l'API est down, on sort (fail fast).
    let catalog = match api_client.get_catalog().await {
        Ok(c) => {
            info!(
                classes = c.classes.len(),
                items = c.shop_items.len(),
                levels = c.level_table.len(),
                "Catalogue Coude recupere depuis l'API"
            );
            Arc::new(c)
        }
        Err(e) => {
            eprintln!("Erreur fatale: impossible de recuperer le catalogue Coude depuis l'API: {e}");
            std::process::exit(1);
        }
    };

    let mut client = Client::builder(config.base().discord_token.as_str(), intents)
        .event_handler(Handler)
        .cache_settings(sentinel_shared::cache_settings::minimal())
        .await
        .expect("Erreur creation du client Discord");

    {
        let mut data = client.data.write().await;
        data.insert::<ApiClientKey>(Arc::clone(&base_api));
        data.insert::<GrpcClientKey>(Arc::clone(&grpc));
        data.insert::<GameApiKey>(api_client);
        data.insert::<catalog::CatalogCacheKey>(catalog);
    }

    // Heartbeat via shared
    spawn_heartbeat(base_api);

    if let Err(e) = sentinel_shared::shard_launcher::start_bot(&mut client).await {
        eprintln!("Erreur fatale : {e}");
    }
}
