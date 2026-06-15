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
mod vote;

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

/// Cree une carte de vote MANUELLE (commande `/card` du module moderation).
/// Reutilise le flux de vote automod (review en base, boutons, finalisation
/// admin), poste dans le salon de review automod (`log_channel_id`), et
/// affiche le contexte avant ET apres le message cible.
///
/// `action_str` ∈ {warn, delete, mute, ban}. Retourne une erreur lisible
/// (a renvoyer au moderateur) si la config est incomplete.
pub async fn create_manual_vote_card(
    ctx: &Context,
    target: &Message,
    action_str: &str,
    reason: &str,
    context_count: u8,
    moderator_name: &str,
) -> Result<(), String> {
    use crate::shared::api_client::BaseApiClient;

    let guild_id = target.guild_id.map(|g| g.to_string()).unwrap_or_default();
    let api = {
        let data = ctx.data.read().await;
        data.get::<crate::shared::heartbeat::ApiClientKey>()
            .cloned()
            .ok_or_else(|| "API indisponible.".to_string())?
    };
    let cfg = api
        .get_guild_config_for(&guild_id, MODULE_BOT_NAME)
        .await
        .unwrap_or_default();

    let review_channel_id = BaseApiClient::config_u64(&cfg, "log_channel_id", 0);
    if review_channel_id == 0 {
        return Err(
            "Aucun salon de review automod configure (parametre `log_channel_id`).".to_string(),
        );
    }
    let deadline_hours = BaseApiClient::config_u64(&cfg, "vote_deadline_hours", 72) as i64;
    let thread_enabled = BaseApiClient::config_bool(&cfg, "vote_thread_enabled", true);
    let discussion_enabled = BaseApiClient::config_bool(&cfg, "discussion_channel_enabled", false);

    let action = match action_str {
        "warn" => api_client::Action::Warn,
        "delete" => api_client::Action::Delete,
        "mute" => api_client::Action::Mute,
        "ban" => api_client::Action::Ban,
        _ => return Err("Action invalide.".to_string()),
    };

    vote::post_manual_vote_card(
        ctx,
        target,
        &action,
        reason,
        review_channel_id,
        deadline_hours,
        context_count,
        thread_enabled,
        moderator_name,
        discussion_enabled,
    )
    .await;
    Ok(())
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
        || custom_id.starts_with(vote::VOTE_PREFIX)
        || custom_id.starts_with(vote::FINALIZE_PREFIX)
        || custom_id.starts_with(vote::DISCUSSION_PREFIX)
}

/// Handle a component interaction (review/vote button click).
pub async fn on_component(ctx: &Context, component: &serenity::model::application::ComponentInteraction) {
    if !is_module_enabled_or_reply_component(ctx, component, MODULE_BOT_NAME).await {
        return;
    }
    let cid = component.data.custom_id.as_str();
    if cid.starts_with(vote::VOTE_PREFIX) {
        vote::handle_vote_button(ctx, component).await;
    } else if cid.starts_with(vote::FINALIZE_PREFIX) {
        vote::handle_finalize_button(ctx, component).await;
    } else if cid.starts_with(vote::DISCUSSION_PREFIX) {
        vote::handle_discussion_button(ctx, component).await;
    } else {
        review::handle_review_button(ctx, component).await;
    }
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
                    vote::handle_decided_event(&ctx, &payload).await;
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
