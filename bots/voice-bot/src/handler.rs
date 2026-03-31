use std::sync::Arc;

use dashmap::DashMap;
use serenity::async_trait;
use serenity::model::application::Interaction;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::model::id::ChannelId;
use serenity::model::voice::VoiceState;
use serenity::prelude::*;
use tracing::info;

use sentinel_shared::heartbeat::register_guilds;

use crate::config::Config;
use crate::state::{AfkTracker, CooldownTracker, FloodTracker, PendingChannels, VoteTracker};

// ── TypeMap keys (bot-specific) ──

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

pub struct AfkTrackerKey;
impl TypeMapKey for AfkTrackerKey {
    type Value = Arc<AfkTracker>;
}

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!(bot = %ready.user.name, "Voice bot connecte");
        register_guilds(&ctx, &ready).await;

        // Lancer le sweep AFK en arriere-plan
        crate::tasks::spawn_afk_sweep(ctx.clone());
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
