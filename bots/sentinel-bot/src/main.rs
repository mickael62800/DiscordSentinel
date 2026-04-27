// Phase 1 — Quick wins : jemalloc en allocateur global (Linux/macOS).
#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

mod config;
mod handler;
mod modules;
mod sync;

use std::sync::Arc;

use serenity::prelude::*;
use tracing::info;

use sentinel_shared::config::BotConfig;
use sentinel_shared::grpc_client::{GrpcClientKey, SentinelGrpcClient};
use sentinel_shared::heartbeat::{ApiClientKey, spawn_heartbeat};

use crate::config::Config;
use crate::handler::Handler;
use crate::modules::{audit, automod, blackjack, community, coude, moderation, progression, security, slot, tickets, voice};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let config = Config::from_env();
    info!("Demarrage de Sentinel Bot (unifie)");

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
        | GatewayIntents::GUILD_MODERATION
        | GatewayIntents::GUILD_MESSAGE_REACTIONS;

    let mut client = Client::builder(config.discord_token(), intents)
        .event_handler(Handler)
        .cache_settings(sentinel_shared::cache_settings::full())
        .await
        .expect("Erreur creation client Discord");

    // Insertion des TypeMapKeys : chaque module gere son propre init. main.rs
    // reste un simple orchestrateur.
    {
        let mut data = client.data.write().await;
        data.insert::<ApiClientKey>(Arc::clone(&api));
        data.insert::<GrpcClientKey>(Arc::clone(&grpc));

        progression::init_typemap(&mut data, &api, &grpc);
        blackjack::init_typemap(&mut data, &api, &grpc);
        slot::init_typemap(&mut data);
        community::init_typemap(&mut data, &api, &grpc);
        security::init_typemap(&mut data, &api, &grpc);
        automod::init_typemap(&mut data);
        audit::init_typemap(&mut data);
        moderation::init_typemap(&mut data, &api, &grpc);
        tickets::init_typemap(&mut data);

        // Coude et voice font des appels API async au boot (catalogue / channels).
        coude::init_typemap(&mut data, &api, &grpc).await;
        voice::init_typemap(&mut data, &api, &grpc).await;
    }

    spawn_heartbeat(api);

    info!("Sentinel Bot pret");

    if let Err(e) = sentinel_shared::shard_launcher::start_bot(&mut client).await {
        eprintln!("Erreur fatale : {e}");
    }
}
