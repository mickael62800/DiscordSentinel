mod api_client;
mod config;
mod handler;
mod handlers;
mod interactions;
mod state;
mod utils;

use std::sync::Arc;

use dashmap::DashMap;
use serenity::prelude::*;
use tracing::info;

use crate::api_client::ApiClient;
use crate::config::Config;
use crate::handler::{
    ApiClientKey, ConfigKey, CooldownTrackerKey, FloodTrackerKey, Handler, MembersToVoiceMapKey,
    PendingChannelsKey, TextToVoiceMapKey, VoiceOwnerMapKey, VoteTrackerKey,
};
use crate::state::{CooldownTracker, FloodTracker, PendingChannels, VoteTracker};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let config = Config::from_env();

    info!(api_url = %config.api_base_url, "Demarrage du voice bot");

    let intents = GatewayIntents::GUILDS
        | GatewayIntents::GUILD_VOICE_STATES
        | GatewayIntents::GUILD_MEMBERS
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    let api_client = ApiClient::new(&config);

    // Charger les salons existants depuis l'API au demarrage
    let text_to_voice: Arc<DashMap<serenity::model::id::ChannelId, serenity::model::id::ChannelId>> = Arc::new(DashMap::new());
    let members_to_voice: Arc<DashMap<serenity::model::id::ChannelId, serenity::model::id::ChannelId>> = Arc::new(DashMap::new());
    let voice_owner: Arc<DashMap<serenity::model::id::ChannelId, serenity::model::id::UserId>> = Arc::new(DashMap::new());

    let guild_id_str = config.guild_id.to_string();
    match api_client.list_channels(&guild_id_str).await {
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

    let mut client = Client::builder(&config.discord_token, intents)
        .event_handler(Handler)
        .await
        .expect("Erreur creation du client Discord");

    // Heartbeat task
    let api_for_heartbeat = ApiClient::new(&config);

    {
        let mut data = client.data.write().await;
        data.insert::<ApiClientKey>(api_client);
        data.insert::<ConfigKey>(config);
        data.insert::<FloodTrackerKey>(Arc::new(FloodTracker::new()));
        data.insert::<VoteTrackerKey>(Arc::new(VoteTracker::new()));
        data.insert::<CooldownTrackerKey>(Arc::new(CooldownTracker::new()));
        data.insert::<PendingChannelsKey>(Arc::new(PendingChannels::new()));
        data.insert::<TextToVoiceMapKey>(text_to_voice);
        data.insert::<MembersToVoiceMapKey>(members_to_voice);
        data.insert::<VoiceOwnerMapKey>(voice_owner);
    }

    tokio::spawn(async move {
        loop {
            if let Err(e) = api_for_heartbeat.heartbeat("voice-bot").await {
                tracing::warn!("Heartbeat failed: {}", e);
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
        }
    });

    if let Err(e) = client.start().await {
        eprintln!("Erreur fatale : {e}");
    }
}
