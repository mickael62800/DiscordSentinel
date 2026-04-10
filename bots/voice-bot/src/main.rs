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
use sentinel_shared::heartbeat::{ApiClientKey, spawn_heartbeat};

use crate::api_client::ApiClient;
use crate::config::Config;
use crate::handler::{
    AfkTrackerKey, ConfigKey, CooldownTrackerKey, FloodTrackerKey, Handler, MembersToVoiceMapKey,
    PendingChannelsKey, SessionCardKey, TextToVoiceMapKey, VoiceOwnerMapKey, VoteTrackerKey,
};
use crate::state::{AfkTracker, CooldownTracker, FloodTracker, PendingChannels, VoteTracker};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let config = Config::from_env();

    info!(api_url = %config.api_base_url(), "Demarrage du voice bot");

    let api = Arc::new(BaseApiClient::new(&config, "voice-bot"));
    let voice_api = ApiClient::new(api.clone());

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
            for ch in &channels {
                let voice_id = serenity::model::id::ChannelId::new(
                    ch.channel_id.parse::<u64>().unwrap_or(0),
                );
                let owner_id = serenity::model::id::UserId::new(
                    ch.owner_id.parse::<u64>().unwrap_or(0),
                );

                voice_owner.insert(voice_id, owner_id);

                if let Some(ref tid) = ch.text_channel_id {
                    if let Ok(id) = tid.parse::<u64>() {
                        text_to_voice.insert(serenity::model::id::ChannelId::new(id), voice_id);
                    }
                }
                if let Some(ref mid) = ch.members_channel_id {
                    if let Ok(id) = mid.parse::<u64>() {
                        members_to_voice.insert(serenity::model::id::ChannelId::new(id), voice_id);
                    }
                }
            }
            info!(count = channels.len(), "Salons temporaires restaures depuis l'API");
        }
        Err(e) => {
            tracing::warn!(error = %e, "Impossible de charger les salons existants depuis l'API");
        }
    }

    let mut client = Client::builder(config.discord_token(), intents)
        .event_handler(Handler)
        .cache_settings(sentinel_shared::cache_settings::full())
        .await
        .expect("Erreur creation du client Discord");

    {
        let mut data = client.data.write().await;
        data.insert::<ApiClientKey>(api.clone());
        data.insert::<ConfigKey>(config);
        data.insert::<FloodTrackerKey>(Arc::new(FloodTracker::new()));
        data.insert::<VoteTrackerKey>(Arc::new(VoteTracker::new()));
        data.insert::<CooldownTrackerKey>(Arc::new(CooldownTracker::new()));
        data.insert::<PendingChannelsKey>(Arc::new(PendingChannels::new()));
        data.insert::<TextToVoiceMapKey>(text_to_voice);
        data.insert::<MembersToVoiceMapKey>(members_to_voice);
        data.insert::<VoiceOwnerMapKey>(voice_owner);
        data.insert::<AfkTrackerKey>(Arc::new(AfkTracker::new()));
        data.insert::<SessionCardKey>(Arc::new(DashMap::new()));
    }

    spawn_heartbeat(api);

    if let Err(e) = sentinel_shared::shard_launcher::start_bot(&mut client).await {
        eprintln!("Erreur fatale : {e}");
    }
}
