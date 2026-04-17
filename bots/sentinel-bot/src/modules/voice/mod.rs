//! Module voice — salons vocaux dynamiques + panels + vote-kick
//! (ex voice-bot).

pub mod api_client;
pub mod config;
pub mod embeds;
pub mod handlers;
pub mod interactions;
pub mod session_card;
pub mod state;
pub mod tasks;

use std::sync::Arc;

use dashmap::DashMap;
use serenity::all::{CommandInteraction, ComponentInteraction, Context, CreateCommand};
use serenity::model::application::ModalInteraction;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::model::id::{ChannelId, UserId};
use serenity::model::voice::VoiceState;
use serenity::prelude::*;
use tracing::{info, warn};

use sentinel_shared::api_client::BaseApiClient;
use sentinel_shared::discord_helpers::is_module_enabled;

use api_client::{ApiClient, VoiceConfigResponse, VoiceThemeResponse};
use config::Config;
use state::{AfkTracker, CooldownTracker, FloodTracker, VoteTracker};

// ── TypeMapKeys ──

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

pub struct TextToVoiceMapKey;
impl TypeMapKey for TextToVoiceMapKey {
    type Value = Arc<DashMap<ChannelId, ChannelId>>;
}

pub struct MembersToVoiceMapKey;
impl TypeMapKey for MembersToVoiceMapKey {
    type Value = Arc<DashMap<ChannelId, ChannelId>>;
}

pub struct VoiceOwnerMapKey;
impl TypeMapKey for VoiceOwnerMapKey {
    type Value = Arc<DashMap<ChannelId, UserId>>;
}

pub struct AfkTrackerKey;
impl TypeMapKey for AfkTrackerKey {
    type Value = Arc<AfkTracker>;
}

pub struct VoiceConfigKey;
impl TypeMapKey for VoiceConfigKey {
    type Value = VoiceConfigResponse;
}

pub struct ThemeCacheKey;
impl TypeMapKey for ThemeCacheKey {
    type Value = Arc<Vec<VoiceThemeResponse>>;
}

pub struct SessionCardKey;
impl TypeMapKey for SessionCardKey {
    type Value = Arc<DashMap<ChannelId, session_card::SessionCard>>;
}

// ── Slash commands (vide — voice est component-based) ──

pub fn register_commands() -> Vec<CreateCommand> {
    vec![]
}

#[allow(dead_code)]
pub async fn handle_command(_ctx: &Context, _command: &CommandInteraction) {
    // Voice-bot n'a pas de commandes slash — tout est via composants.
}

// ── Component interactions ──

pub fn handles_component(cid: &str) -> bool {
    matches!(
        cid,
        "btn_hide" | "btn_lock" | "btn_limit" | "btn_rename" | "btn_status"
            | "select_invite" | "btn_kick" | "select_kick" | "btn_ban" | "select_ban"
            | "btn_coadmin" | "select_coadmin"
            | "btn_transfer" | "select_transfer"
            | "btn_queue"
            | "select_votekick" | "votekick_yes" | "votekick_no"
    ) || cid.starts_with("limit_")
        || cid.starts_with("ban_duration_")
        || cid.starts_with("queue_accept_")
        || cid.starts_with("queue_refuse_")
        || cid.starts_with("btn_claim_ownership_")
}

pub async fn on_component(ctx: &Context, component: &ComponentInteraction) {
    if let Some(guild_id) = component.guild_id {
        if !is_module_enabled(ctx, &guild_id.to_string()).await {
            return;
        }
    }
    interactions::handle_component(ctx, component).await;
}

pub fn handles_modal(cid: &str) -> bool {
    matches!(cid, "modal_rename" | "modal_status" | "modal_limit")
}

pub async fn on_modal(ctx: &Context, modal: &ModalInteraction) {
    interactions::handle_modal(ctx, modal).await;
}

// ── Event handlers ──

pub async fn on_message(ctx: &Context, msg: &Message) {
    if let Some(guild_id) = msg.guild_id {
        if !is_module_enabled(ctx, &guild_id.to_string()).await {
            return;
        }
    }
    handlers::message::handle_message(ctx, msg).await;
}

pub async fn on_voice_state_update(ctx: &Context, old: &Option<VoiceState>, new: &VoiceState) {
    if let Some(guild_id) = new.guild_id {
        if !is_module_enabled(ctx, &guild_id.to_string()).await {
            return;
        }
    }
    handlers::voice::handle_voice_state_update(ctx, old, new).await;
}

/// Initialisation appelee depuis ready (reconcile channels + spawn background tasks).
pub async fn on_ready(ctx: &Context, ready: &Ready) {
    reconcile_voice_channels(ctx, ready).await;
    tasks::spawn_afk_sweep(ctx.clone());
}

/// Charge les salons vocaux ouverts depuis l'API et verifie leur existence Discord.
async fn reconcile_voice_channels(ctx: &Context, ready: &Ready) {
    let (api, voice_owner, text_to_voice, members_to_voice) = {
        let data = ctx.data.read().await;
        let api = match api_client::ApiClient::from_data(&data) {
            Some(a) => a,
            None => {
                warn!("reconcile_voice_channels: ApiClient absent, skip");
                return;
            }
        };
        let voice_owner: Option<Arc<DashMap<ChannelId, UserId>>> =
            data.get::<VoiceOwnerMapKey>().cloned();
        let text_to_voice: Option<Arc<DashMap<ChannelId, ChannelId>>> =
            data.get::<TextToVoiceMapKey>().cloned();
        let members_to_voice: Option<Arc<DashMap<ChannelId, ChannelId>>> =
            data.get::<MembersToVoiceMapKey>().cloned();
        (api, voice_owner, text_to_voice, members_to_voice)
    };

    let mut reloaded = 0usize;
    let mut ghosts_closed = 0usize;

    for guild in &ready.guilds {
        let guild_id_str = guild.id.to_string();
        let channels = match api.list_channels(&guild_id_str).await {
            Ok(list) => list,
            Err(e) => {
                warn!(error = %e, guild_id = %guild_id_str, "reconcile: echec list_channels");
                continue;
            }
        };

        for ch in channels {
            let voice_cid = match ch.channel_id.parse::<u64>() {
                Ok(n) => ChannelId::new(n),
                Err(_) => continue,
            };

            let exists = voice_cid.to_channel(&ctx.http).await.is_ok();
            if !exists {
                if let Err(e) = api.delete_channel(&ch.channel_id).await {
                    warn!(
                        error = %e,
                        channel_id = %ch.channel_id,
                        "reconcile: echec close_channel fantome"
                    );
                } else {
                    ghosts_closed += 1;
                }
                continue;
            }

            if let Some(ref map) = voice_owner {
                if let Ok(owner_id) = ch.owner_id.parse::<u64>() {
                    map.insert(voice_cid, UserId::new(owner_id));
                }
            }
            if let Some(ref map) = text_to_voice {
                if let Some(text_id_str) = ch.text_channel_id.as_deref() {
                    if let Ok(n) = text_id_str.parse::<u64>() {
                        map.insert(ChannelId::new(n), voice_cid);
                    }
                }
            }
            if let Some(ref map) = members_to_voice {
                if let Some(members_id_str) = ch.members_channel_id.as_deref() {
                    if let Ok(n) = members_id_str.parse::<u64>() {
                        map.insert(ChannelId::new(n), voice_cid);
                    }
                }
            }
            reloaded += 1;
        }
    }

    info!(reloaded, ghosts_closed, "reconcile_voice_channels termine");
}

/// Initialise les TypeMapKeys voice dans `data`. Appele depuis main.rs.
pub async fn init_typemap(
    data: &mut serenity::prelude::TypeMap,
    api: &Arc<BaseApiClient>,
    grpc: &Arc<sentinel_shared::grpc_client::SentinelGrpcClient>,
) {
    let config = Config::from_env();

    if config.guild_id == 0 {
        info!("Voice module: VOICE_GUILD_ID non defini, module desactive");
    }

    let voice_api = ApiClient::new(Arc::clone(api), Arc::clone(grpc));

    let text_to_voice: Arc<DashMap<ChannelId, ChannelId>> = Arc::new(DashMap::new());
    let members_to_voice: Arc<DashMap<ChannelId, ChannelId>> = Arc::new(DashMap::new());
    let voice_owner: Arc<DashMap<ChannelId, UserId>> = Arc::new(DashMap::new());

    if config.guild_id > 0 {
        let guild_id_str = config.guild_id.to_string();
        if let Ok(channels) = voice_api.list_channels(&guild_id_str).await {
            for ch in &channels {
                let voice_id = match ch.channel_id.parse::<u64>() {
                    Ok(id) if id > 0 => ChannelId::new(id),
                    _ => continue,
                };
                let owner_id = match ch.owner_id.parse::<u64>() {
                    Ok(id) if id > 0 => UserId::new(id),
                    _ => continue,
                };
                voice_owner.insert(voice_id, owner_id);

                if let Some(ref tid) = ch.text_channel_id {
                    if let Ok(id) = tid.parse::<u64>() {
                        if id > 0 {
                            text_to_voice.insert(ChannelId::new(id), voice_id);
                        }
                    }
                }
                if let Some(ref mid) = ch.members_channel_id {
                    if let Ok(id) = mid.parse::<u64>() {
                        if id > 0 {
                            members_to_voice.insert(ChannelId::new(id), voice_id);
                        }
                    }
                }
            }
        }
    }

    let cooldown_tracker = Arc::new(CooldownTracker::new());
    let flood_tracker = Arc::new(FloodTracker::new());

    let voice_config = if config.guild_id > 0 {
        match voice_api.get_voice_config(&config.guild_id.to_string()).await {
            Ok(cfg) => {
                cooldown_tracker.set_cooldown_secs(cfg.creation_cooldown_secs);
                flood_tracker.set_thresholds(cfg.flood_max_messages, cfg.flood_time_window_secs);
                cfg
            }
            Err(_) => default_voice_config(),
        }
    } else {
        default_voice_config()
    };

    let themes = if config.guild_id > 0 {
        Arc::new(
            voice_api
                .list_themes(&config.guild_id.to_string())
                .await
                .unwrap_or_default(),
        )
    } else {
        Arc::new(vec![])
    };

    data.insert::<ConfigKey>(config);
    data.insert::<FloodTrackerKey>(flood_tracker);
    data.insert::<VoteTrackerKey>(Arc::new(VoteTracker::new()));
    data.insert::<CooldownTrackerKey>(cooldown_tracker);
    data.insert::<TextToVoiceMapKey>(text_to_voice);
    data.insert::<MembersToVoiceMapKey>(members_to_voice);
    data.insert::<VoiceOwnerMapKey>(voice_owner);
    data.insert::<AfkTrackerKey>(Arc::new(AfkTracker::new()));
    data.insert::<VoiceConfigKey>(voice_config);
    data.insert::<ThemeCacheKey>(themes);
    data.insert::<SessionCardKey>(Arc::new(DashMap::new()));
}

fn default_voice_config() -> VoiceConfigResponse {
    VoiceConfigResponse {
        creation_cooldown_secs: 5,
        flood_max_messages: 5,
        flood_time_window_secs: 5,
        empty_cleanup_delay_secs: 2,
        flood_mute_duration_secs: 30,
        vote_kick_timeout_secs: 60,
    }
}
