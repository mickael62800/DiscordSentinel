//! Module automod — detection spam/insultes/liens/phishing + slowmode adaptatif.
//! Migre depuis automod-bot.

pub const MODULE_BOT_NAME: &str = "automod-bot";

mod api_client;
pub mod adaptive_slowmode;
pub mod automod_cmd;
mod backend;
mod config;
pub mod detectors;
mod message_handler;
mod review;

use std::sync::Arc;
use std::time::Instant;

use dashmap::DashMap;
use serenity::all::CreateCommand;
use serenity::model::application::CommandInteraction;
use serenity::model::channel::Message;
use serenity::model::id::{ChannelId, MessageId, UserId};
use serenity::prelude::*;
use tracing::{info, warn};

use crate::shared::discord_helpers::{
    is_module_enabled, is_module_enabled_or_reply_command, is_module_enabled_or_reply_component,
};

use self::adaptive_slowmode::SlowmodeTracker;

// ══════════════════════════════════════════════════════════════════════
// TypeMapKeys
// ══════════════════════════════════════════════════════════════════════

/// Deduplication des messages deja traites (avec timestamp pour cleanup)
pub struct ProcessedMessagesKey;

impl TypeMapKey for ProcessedMessagesKey {
    type Value = Arc<DashMap<MessageId, Instant>>;
}

/// Flood tracker : (channel_id, user_id) -> liste de timestamps
pub struct FloodTrackerKey;

impl TypeMapKey for FloodTrackerKey {
    type Value = Arc<DashMap<(ChannelId, UserId), Vec<Instant>>>;
}

pub struct SlowmodeTrackerKey;
impl TypeMapKey for SlowmodeTrackerKey {
    type Value = SlowmodeTracker;
}

// ══════════════════════════════════════════════════════════════════════
// Module interface (register_commands, handle_command, handles_component)
// ══════════════════════════════════════════════════════════════════════

/// Insere les TypeMapKeys du module automod.
pub fn init_typemap(data: &mut serenity::prelude::TypeMap) {
    use dashmap::DashMap;
    data.insert::<ProcessedMessagesKey>(Arc::new(DashMap::new()));
    data.insert::<FloodTrackerKey>(Arc::new(DashMap::new()));
    data.insert::<SlowmodeTrackerKey>(adaptive_slowmode::SlowmodeTracker::new(30));
}

pub fn register_commands() -> Vec<CreateCommand> {
    vec![automod_cmd::register()]
}

pub async fn handle_command(ctx: &Context, command: &CommandInteraction) {
    if !is_module_enabled_or_reply_command(ctx, command, MODULE_BOT_NAME).await {
        return;
    }
    automod_cmd::handle(ctx, command).await;
}

/// Returns true if the component custom_id belongs to automod review buttons.
pub fn handles_component(custom_id: &str) -> bool {
    custom_id.starts_with(AM_PREFIX)
}

/// Handle a component interaction (review button click).
pub async fn on_component(ctx: &Context, component: &serenity::model::application::ComponentInteraction) {
    if !is_module_enabled_or_reply_component(ctx, component, MODULE_BOT_NAME).await {
        return;
    }
    review::handle_review_button(ctx, component).await;
}

// ══════════════════════════════════════════════════════════════════════
// on_ready — background tasks (slowmode deactivation + cache cleanup)
// ══════════════════════════════════════════════════════════════════════

/// Spawn background tasks for automod (slowmode deactivation + cache cleanup).
/// Called from the sentinel handler's ready event.
pub fn spawn_background_tasks(ctx: &Context) {
    // Background task : desactiver le slowmode adaptatif quand l'activite retombe
    let ctx_clone = ctx.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
            let data = ctx_clone.data.read().await;
            if let Some(tracker) = data.get::<SlowmodeTrackerKey>() {
                let to_deactivate = tracker.channels_to_deactivate(15);
                drop(data);
                for channel_id in to_deactivate {
                    let edit = serenity::builder::EditChannel::new().rate_limit_per_user(0);
                    if let Err(e) = channel_id.edit(&ctx_clone.http, edit).await {
                        warn!(error = %e, channel_id = %channel_id, "Echec desactivation slowmode adaptatif");
                    } else {
                        info!(channel_id = %channel_id, "Slowmode adaptatif desactive (activite retombee)");
                    }
                }
            }
        }
    });

    // Redis listener : `automod_review_resolved` depuis web -> edit la carte
    // Discord (greyed-out + footer "via web") + applique l'action (warn/mute/
    // ban/delete). Idempotent : skip si actor.source != "web".
    let ctx_redis = ctx.clone();
    tokio::spawn(async move {
        let consumer = crate::shared::event_bus::default_consumer_name();
        crate::shared::event_bus::listen_stream_group(
            "automod-bot".to_string(),
            consumer,
            move |payload| {
                let ctx = ctx_redis.clone();
                async move {
                    review::handle_redis_event(&ctx, &payload).await;
                }
            },
        )
        .await;
    });

    // Background cleanup : purge des caches processed + flood
    // tracker toutes les 5 minutes.
    let ctx_clean = ctx.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(300)).await;
            let data = ctx_clean.data.read().await;
            let now = Instant::now();

            if let Some(processed) = data.get::<ProcessedMessagesKey>() {
                let before = processed.len();
                processed.retain(|_, ts| now.duration_since(*ts).as_secs() < 300);
                let removed = before.saturating_sub(processed.len());
                if removed > 0 {
                    info!(removed, remaining = processed.len(), "Purge background processed cache");
                }
            }

            if let Some(tracker) = data.get::<FloodTrackerKey>() {
                let before = tracker.len();
                tracker.retain(|_, ts| {
                    ts.last()
                        .map(|t| now.duration_since(*t).as_secs() < 600)
                        .unwrap_or(false)
                });
                let removed = before.saturating_sub(tracker.len());
                if removed > 0 {
                    info!(removed, remaining = tracker.len(), "Purge background flood tracker");
                }
            }
        }
    });
}

// ══════════════════════════════════════════════════════════════════════
// on_message — main automod logic (extracted from handler.rs message())
// ══════════════════════════════════════════════════════════════════════

/// Custom ID format : `am_{action}:{guild_id}:{channel_id}:{message_id}:{user_id}`
/// action = w (warn) | d (delete) | m (mute) | b (ban) | i (ignore)
const AM_PREFIX: &str = "am_";

/// Valeur par defaut utilisee par le review handler quand la config guild est absente.
const DEFAULT_MUTE_DURATION_SECS: u64 = 600;

/// Main automod message handler. Called from the sentinel handler's message event.
pub async fn on_message(ctx: &Context, msg: &Message) {
    if let Some(guild_id) = msg.guild_id {
        if !is_module_enabled(ctx, &guild_id.to_string(), MODULE_BOT_NAME).await {
            return;
        }
    }
    message_handler::process(ctx, msg).await
}
