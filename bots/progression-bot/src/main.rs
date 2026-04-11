// Phase 1 — Quick wins : jemalloc en allocateur global (Linux/macOS).
// Sur Windows MSVC, on retombe sur l'allocateur système.
#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

mod api_client;
mod badges;
mod commands;
mod config;
mod handler;
mod level_channel;
mod multipliers;
mod streaks;
mod tracker;
mod xp_cooldown;

use std::sync::Arc;

use serenity::prelude::*;
use tracing::info;

use sentinel_shared::api_client::BaseApiClient;
use sentinel_shared::config::BotConfig;
use sentinel_shared::grpc_client::{GrpcClientKey, SentinelGrpcClient};
use sentinel_shared::heartbeat::{ApiClientKey, spawn_heartbeat};

use crate::api_client::ApiClient;
use crate::config::Config;
use crate::handler::{Handler, StatsApiKey, TrackerKey, XpCooldownKey, StreakTrackerKey, RewardsCacheKey, RewardsCache};
use crate::streaks::StreakTracker;
use crate::tracker::StatsTracker;
use crate::xp_cooldown::XpCooldown;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let config = Config::from_env("PROGRESSION_DISCORD_TOKEN");

    info!(api_url = %config.api_base_url(), "Demarrage du progression bot");

    let base = Arc::new(BaseApiClient::new(&config, "progression-bot"));

    // Phase 7A — pilote gRPC : connexion lazy au serveur tonic de l'API.
    // L'URL est lue depuis GRPC_API_URL (defaut http://127.0.0.1:50051).
    // L'auth Bearer reuse API_KEY. Si la connexion echoue au demarrage,
    // le circuit breaker prendra le relais et chaque appel sera proprement
    // court-circuite — le bot ne crashe pas.
    let grpc = match SentinelGrpcClient::from_env().await {
        Ok(c) => Arc::new(c),
        Err(e) => {
            tracing::error!(error = %e, "Echec init SentinelGrpcClient — fallback HTTP partiel");
            // On panique ici plutot que de demarrer le bot dans un etat
            // degrade silencieux : la migration gRPC est obligatoire pour ce bot.
            panic!("SentinelGrpcClient: {e}");
        }
    };

    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILD_VOICE_STATES
        | GatewayIntents::GUILD_MEMBERS
        | GatewayIntents::GUILDS;

    let mut client = Client::builder(config.discord_token(), intents)
        .event_handler(Handler)
        .cache_settings(sentinel_shared::cache_settings::small())
        .await
        .expect("Erreur creation du client Discord");

    {
        let mut data = client.data.write().await;
        data.insert::<ApiClientKey>(Arc::clone(&base));
        data.insert::<GrpcClientKey>(Arc::clone(&grpc));
        data.insert::<StatsApiKey>(ApiClient::new(Arc::clone(&base), Arc::clone(&grpc)));
        data.insert::<TrackerKey>(StatsTracker::new());
        data.insert::<XpCooldownKey>(XpCooldown::new());
        data.insert::<StreakTrackerKey>(StreakTracker::new());
        data.insert::<RewardsCacheKey>(Arc::new(RewardsCache::new()));
    }

    spawn_heartbeat(Arc::clone(&base));

    if let Err(e) = sentinel_shared::shard_launcher::start_bot(&mut client).await {
        eprintln!("Erreur fatale : {e}");
    }
}
