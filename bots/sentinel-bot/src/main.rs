// Phase 1 — Quick wins : jemalloc en allocateur global (Linux/macOS).
#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

mod config;
mod handler;
mod modules;

use std::sync::Arc;

use dashmap::DashMap;
use serenity::prelude::*;
use tracing::info;

use sentinel_shared::config::BotConfig;
use sentinel_shared::grpc_client::{GrpcClientKey, SentinelGrpcClient};
use sentinel_shared::heartbeat::{ApiClientKey, spawn_heartbeat};

use crate::config::Config;
use crate::handler::Handler;
use crate::modules::{audit, automod, blackjack, community, coude, moderation, progression, security, tickets, voice};

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
        | GatewayIntents::GUILD_MODERATION;

    let mut client = Client::builder(config.discord_token(), intents)
        .event_handler(Handler)
        .cache_settings(sentinel_shared::cache_settings::full())
        .await
        .expect("Erreur creation client Discord");

    {
        let mut data = client.data.write().await;
        data.insert::<ApiClientKey>(api.clone());
        data.insert::<GrpcClientKey>(grpc.clone());
        // Progression TypeMapKeys
        data.insert::<progression::StatsApiKey>(
            progression::api_client::ApiClient::new(Arc::clone(&api), Arc::clone(&grpc)),
        );
        data.insert::<progression::TrackerKey>(progression::tracker::StatsTracker::new());
        data.insert::<progression::XpCooldownKey>(progression::xp_cooldown::XpCooldown::new());
        data.insert::<progression::StreakTrackerKey>(progression::streaks::StreakTracker::new());
        data.insert::<progression::RewardsCacheKey>(Arc::new(progression::RewardsCache::new()));
        // Blackjack TypeMapKeys
        data.insert::<blackjack::GameApiKey>(
            blackjack::api_client::ApiClient::new(Arc::clone(&api), Arc::clone(&grpc)),
        );
        data.insert::<blackjack::ChannelManagerKey>(
            Arc::new(blackjack::channel_manager::ChannelManager::new()),
        );
        // Community TypeMapKeys
        data.insert::<community::RolesApiKey>(
            community::api_client::ApiClient::new(Arc::clone(&api), Arc::clone(&grpc)),
        );
        data.insert::<community::CooldownKey>(Arc::new(community::cooldown::InteractionCooldown::new()));
        data.insert::<community::TempRoleKey>(community::temp_roles::TempRoleTracker::new());
        data.insert::<community::SponsorshipKey>(community::sponsorship::SponsorshipTracker::new());
        // Security TypeMapKeys
        let sec_config = security::SecurityConfig::from_env();
        data.insert::<security::SecurityApiKey>(
            security::api_client::ApiClient::new(Arc::clone(&api), Arc::clone(&grpc)),
        );
        data.insert::<security::RaidDetectorKey>(
            security::detectors::raid_detector::RaidDetector::new(
                sec_config.raid_join_threshold,
                sec_config.raid_join_window_secs,
            ),
        );
        data.insert::<security::AccountCheckerKey>(
            security::detectors::account_checker::AccountChecker::new(sec_config.min_account_age_secs),
        );
        data.insert::<security::QuarantineKey>(
            security::detectors::quarantine::QuarantineManager::new(),
        );
        data.insert::<security::SlowmodeKey>(
            security::detectors::slowmode::SlowmodeManager::new(),
        );
        data.insert::<security::LockdownKey>(
            security::detectors::lockdown::LockdownManager::new(),
        );
        data.insert::<security::RecentJoinsKey>(
            security::detectors::raid_analyzer::RecentJoinsTracker::new(sec_config.raid_join_window_secs),
        );
        data.insert::<security::CaptchaPendingKey>(
            security::detectors::captcha::CaptchaPending::new(),
        );
        data.insert::<security::AltDetectorKey>(
            security::detectors::alt_detector::AltDetector::new(
                sec_config.alt_retention_secs,
                sec_config.alt_name_distance,
                3600,
            ),
        );
        data.insert::<security::SecurityConfigKey>(sec_config);
        // Automod TypeMapKeys
        data.insert::<automod::ProcessedMessagesKey>(Arc::new(DashMap::new()));
        data.insert::<automod::FloodTrackerKey>(Arc::new(DashMap::new()));
        data.insert::<automod::SlowmodeTrackerKey>(automod::adaptive_slowmode::SlowmodeTracker::new(30));
        // Audit TypeMapKeys
        let audit_config = audit::AuditConfig::default();
        data.insert::<audit::MessageCacheKey>(audit::message_cache::MessageCache::new(audit_config.message_cache_size));
        data.insert::<audit::AnomalyDetectorKey>(audit::anomaly::AnomalyDetector::new(
            audit_config.anomaly_window_secs,
            audit::anomaly::AnomalyThresholds {
                mass_ban: audit_config.anomaly_mass_ban_threshold,
                mass_delete: audit_config.anomaly_mass_delete_threshold,
                mass_role_change: audit_config.anomaly_mass_role_threshold,
            },
        ));
        data.insert::<audit::WeeklyTrackerKey>(audit::weekly_report::WeeklyTracker::new());
        data.insert::<audit::ConfigKey>(audit_config);
        data.insert::<audit::WatchedUserIdsKey>(Arc::new(dashmap::DashSet::new()));
        // Moderation TypeMapKeys
        data.insert::<moderation::ModerationApiKey>(Arc::new(
            moderation::api_client::ApiClient::new(Arc::clone(&api), Arc::clone(&grpc)),
        ));
        data.insert::<moderation::PendingActionsKey>(DashMap::new());
        data.insert::<moderation::risk_check::RiskyPendingKey>(DashMap::new());
        // Coude TypeMapKeys
        let coude_api = coude::api_client::ApiClient::new(Arc::clone(&api), Arc::clone(&grpc));
        // Phase 8 : fetch du catalogue Coude depuis l'API au boot. Si l'API
        // est down, on log et on insere un catalogue vide (fail-degraded :
        // le reste du bot continue de tourner).
        let coude_catalog = match coude_api.get_catalog().await {
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
                tracing::warn!(error = %e, "Echec fetch catalogue Coude (fail-degraded, catalogue vide)");
                Arc::new(coude::catalog::CatalogCache {
                    classes: Vec::new(),
                    shop_items: Vec::new(),
                    level_table: Vec::new(),
                    matchmaking_buckets: Vec::new(),
                    anti_theft_items: Vec::new(),
                    max_level: 1,
                    hp_base: 100,
                    hp_per_def: 2,
                })
            }
        };
        data.insert::<coude::GameApiKey>(coude_api);
        data.insert::<coude::CatalogCacheKey>(coude_catalog);
        // Tickets TypeMapKeys
        data.insert::<tickets::config::ConfigKey>(tickets::TicketsConfig::from_env());
        data.insert::<tickets::SlaTrackerKey>(tickets::sla::SlaTracker::new());
    }

    // Voice TypeMapKeys (inseres apres pour que init_typemap puisse acceder au data map).
    voice::init_typemap(&client, Arc::clone(&api), Arc::clone(&grpc)).await;

    spawn_heartbeat(api);

    info!("Sentinel Bot pret");

    if let Err(e) = sentinel_shared::shard_launcher::start_bot(&mut client).await {
        eprintln!("Erreur fatale : {e}");
    }
}
