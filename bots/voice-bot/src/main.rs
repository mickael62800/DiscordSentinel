// Phase 1 — Quick wins : jemalloc en allocateur global (Linux/macOS).
// Sur Windows MSVC, on retombe sur l'allocateur système.
#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

mod api_client;
mod config;
mod embeds;
mod handler;
mod handlers;
mod interactions;
pub mod session_card;
mod state;
mod tasks;

use std::sync::Arc;

use dashmap::DashMap;
use serenity::prelude::*;
use tracing::info;

use sentinel_shared::api_client::BaseApiClient;
use sentinel_shared::config::BotConfig;
use sentinel_shared::grpc_client::{GrpcClientKey, SentinelGrpcClient};
use sentinel_shared::heartbeat::{ApiClientKey, spawn_heartbeat};

use crate::api_client::ApiClient;
use crate::config::Config;
use crate::api_client::VoiceConfigResponse;
use crate::handler::{
    AfkTrackerKey, ConfigKey, CooldownTrackerKey, FloodTrackerKey, Handler, MembersToVoiceMapKey,
    SessionCardKey, TextToVoiceMapKey, VoiceConfigKey, VoiceOwnerMapKey, VoteTrackerKey,
};
use crate::state::{AfkTracker, CooldownTracker, FloodTracker, VoteTracker};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let config = Config::from_env();

    info!(api_url = %config.api_base_url(), "Demarrage du voice bot");

    let api = Arc::new(BaseApiClient::new(&config, "voice-bot"));

    // Phase 7A — gRPC interne (VoiceChannelsService + ModerationService).
    // Phase 7A opt D.2 : passage du pattern OnceLock au classique TypeMap.
    // Le client est insere dans Serenity data et fetche par chaque call site
    // via `data.get::<GrpcClientKey>()`.
    let grpc = match SentinelGrpcClient::from_env().await {
        Ok(c) => Arc::new(c),
        Err(e) => {
            eprintln!("Erreur fatale: impossible d'initialiser SentinelGrpcClient: {e}");
            std::process::exit(1);
        }
    };

    let voice_api = ApiClient::new(api.clone(), Arc::clone(&grpc));

    let intents = GatewayIntents::GUILDS
        | GatewayIntents::GUILD_VOICE_STATES
        | GatewayIntents::GUILD_MEMBERS
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    // Charger les salons existants depuis l'API au demarrage
    let text_to_voice: Arc<DashMap<serenity::model::id::ChannelId, serenity::model::id::ChannelId>> = Arc::new(DashMap::new());
    let members_to_voice: Arc<DashMap<serenity::model::id::ChannelId, serenity::model::id::ChannelId>> = Arc::new(DashMap::new());
    let voice_owner: Arc<DashMap<serenity::model::id::ChannelId, serenity::model::id::UserId>> = Arc::new(DashMap::new());

    let guild_id_str = config.guild_id.to_string();
    match voice_api.list_channels(&guild_id_str).await {
        Ok(channels) => {
            let mut restored = 0usize;
            for ch in &channels {
                // Skip les entrees avec un channel_id ou owner_id non parsable :
                // mettre ChannelId(0)/UserId(0) polluait les DashMap et cassait
                // les lookups ulterieurs.
                let voice_id = match ch.channel_id.parse::<u64>() {
                    Ok(id) if id > 0 => serenity::model::id::ChannelId::new(id),
                    _ => {
                        tracing::warn!(
                            channel_id = %ch.channel_id,
                            "channel_id invalide dans l'API, entree ignoree"
                        );
                        continue;
                    }
                };
                let owner_id = match ch.owner_id.parse::<u64>() {
                    Ok(id) if id > 0 => serenity::model::id::UserId::new(id),
                    _ => {
                        tracing::warn!(
                            owner_id = %ch.owner_id,
                            channel_id = %ch.channel_id,
                            "owner_id invalide dans l'API, entree ignoree"
                        );
                        continue;
                    }
                };

                voice_owner.insert(voice_id, owner_id);

                if let Some(ref tid) = ch.text_channel_id {
                    if let Ok(id) = tid.parse::<u64>() {
                        if id > 0 {
                            text_to_voice.insert(serenity::model::id::ChannelId::new(id), voice_id);
                        }
                    }
                }
                if let Some(ref mid) = ch.members_channel_id {
                    if let Ok(id) = mid.parse::<u64>() {
                        if id > 0 {
                            members_to_voice.insert(serenity::model::id::ChannelId::new(id), voice_id);
                        }
                    }
                }
                restored += 1;
            }
            info!(
                total = channels.len(),
                restored,
                "Salons temporaires restaures depuis l'API"
            );
        }
        Err(e) => {
            tracing::warn!(error = %e, "Impossible de charger les salons existants depuis l'API");
        }
    }

    // Charger la config voice depuis l'API (cooldown, flood, cleanup).
    // En cas d'echec, on garde les defaults — le bot demarre quand meme.
    let cooldown_tracker = Arc::new(CooldownTracker::new());
    let flood_tracker = Arc::new(FloodTracker::new());
    let voice_config = match voice_api.get_voice_config(&guild_id_str).await {
        Ok(cfg) => {
            cooldown_tracker.set_cooldown_secs(cfg.creation_cooldown_secs);
            flood_tracker.set_thresholds(cfg.flood_max_messages, cfg.flood_time_window_secs);
            info!(
                cooldown = cfg.creation_cooldown_secs,
                flood_max = cfg.flood_max_messages,
                flood_window = cfg.flood_time_window_secs,
                cleanup_delay = cfg.empty_cleanup_delay_secs,
                mute_duration = cfg.flood_mute_duration_secs,
                vote_kick_timeout = cfg.vote_kick_timeout_secs,
                "Voice config chargee depuis l'API"
            );
            cfg
        }
        Err(e) => {
            tracing::warn!(error = %e, "Impossible de charger la voice config, defaults utilises");
            VoiceConfigResponse {
                creation_cooldown_secs: 5,
                flood_max_messages: 5,
                flood_time_window_secs: 5,
                empty_cleanup_delay_secs: 2,
                flood_mute_duration_secs: 30,
                vote_kick_timeout_secs: 60,
            }
        }
    };

    let mut client = Client::builder(config.discord_token(), intents)
        .event_handler(Handler)
        .cache_settings(sentinel_shared::cache_settings::full())
        .await
        .expect("Erreur creation du client Discord");

    {
        let mut data = client.data.write().await;
        data.insert::<ApiClientKey>(api.clone());
        data.insert::<GrpcClientKey>(Arc::clone(&grpc));
        data.insert::<ConfigKey>(config);
        data.insert::<FloodTrackerKey>(flood_tracker);
        data.insert::<VoteTrackerKey>(Arc::new(VoteTracker::new()));
        data.insert::<CooldownTrackerKey>(cooldown_tracker);
        data.insert::<TextToVoiceMapKey>(text_to_voice);
        data.insert::<MembersToVoiceMapKey>(members_to_voice);
        data.insert::<VoiceOwnerMapKey>(voice_owner);
        data.insert::<AfkTrackerKey>(Arc::new(AfkTracker::new()));
        data.insert::<VoiceConfigKey>(voice_config);
        data.insert::<SessionCardKey>(Arc::new(DashMap::new()));
    }

    spawn_heartbeat(api);

    if let Err(e) = sentinel_shared::shard_launcher::start_bot(&mut client).await {
        eprintln!("Erreur fatale : {e}");
    }
}
