use std::sync::Arc;

use dashmap::DashMap;
use serenity::async_trait;
use serenity::model::application::Interaction;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::model::id::ChannelId;
use serenity::model::voice::VoiceState;
use serenity::prelude::*;
use tracing::{info, warn};

use crate::api_client::ApiClient;
use crate::config::Config;
use crate::state::{CooldownTracker, FloodTracker, PendingChannels, VoteTracker};

// ── TypeMap keys ──

pub struct ApiClientKey;
impl TypeMapKey for ApiClientKey {
    type Value = ApiClient;
}

pub struct ConfigKey;
impl TypeMapKey for ConfigKey {
    type Value = Config;
}

pub struct FloodTrackerKey;
impl TypeMapKey for FloodTrackerKey {
    type Value = Arc<FloodTracker>;
}

pub struct VoteTrackerKey;
impl TypeMapKey for VoteTrackerKey {
    type Value = Arc<VoteTracker>;
}

pub struct CooldownTrackerKey;
impl TypeMapKey for CooldownTrackerKey {
    type Value = Arc<CooldownTracker>;
}

pub struct PendingChannelsKey;
impl TypeMapKey for PendingChannelsKey {
    type Value = Arc<PendingChannels>;
}

/// Mapping text_channel_id -> voice_channel_id (pour retrouver le vocal depuis le panel admin)
pub struct TextToVoiceMapKey;
impl TypeMapKey for TextToVoiceMapKey {
    type Value = Arc<DashMap<ChannelId, ChannelId>>;
}

/// Mapping members_channel_id -> voice_channel_id (pour retrouver le vocal depuis le panel membres)
pub struct MembersToVoiceMapKey;
impl TypeMapKey for MembersToVoiceMapKey {
    type Value = Arc<DashMap<ChannelId, ChannelId>>;
}

/// Mapping voice_channel_id -> owner_id (cache local pour eviter des appels API)
pub struct VoiceOwnerMapKey;
impl TypeMapKey for VoiceOwnerMapKey {
    type Value = Arc<DashMap<ChannelId, serenity::model::id::UserId>>;
}

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!(bot = %ready.user.name, "Voice bot connecte");

        // Enregistrer les guilds aupres de l'API
        let data = ctx.data.read().await;
        if let Some(api) = data.get::<ApiClientKey>() {
            for guild_status in &ready.guilds {
                let guild_id = guild_status.id;
                if let Ok(guild) = guild_id.to_partial_guild(&ctx.http).await {
                    let member_count = guild.approximate_member_count.unwrap_or(0) as i32;
                    if let Err(e) = api.register_guild(
                        &guild_id.to_string(),
                        &guild.name,
                        member_count,
                    ).await {
                        warn!(error = %e, guild = %guild.name, "Erreur enregistrement guild");
                    } else {
                        info!(guild = %guild.name, "Guild enregistree");
                    }
                }
            }
        }
    }

    async fn voice_state_update(&self, ctx: Context, old: Option<VoiceState>, new: VoiceState) {
        crate::handlers::voice::handle_voice_state_update(&ctx, &old, &new).await;
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        match &interaction {
            Interaction::Component(component) => {
                crate::interactions::handle_component(&ctx, component).await;
            }
            Interaction::Modal(modal) => {
                crate::interactions::handle_modal(&ctx, modal).await;
            }
            _ => {}
        }
    }

    async fn message(&self, ctx: Context, msg: Message) {
        crate::handlers::message::handle_message(&ctx, &msg).await;
    }
}
