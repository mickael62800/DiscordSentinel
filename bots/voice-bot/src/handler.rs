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

use sentinel_shared::api_client::BaseApiClient;
use sentinel_shared::heartbeat::{ApiClientKey, register_guilds};

use crate::config::Config;
use crate::state::{AfkTracker, CooldownTracker, FloodTracker, VoteTracker};

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

/// Carte live de session vocale dans le salon de logs.
/// voice_channel_id -> SessionCard
pub struct SessionCardKey;
impl TypeMapKey for SessionCardKey {
    type Value = Arc<DashMap<ChannelId, crate::session_card::SessionCard>>;
}

/// Au demarrage du bot, recharge les salons vocaux ouverts depuis la
/// DB et verifie leur existence cote Discord :
/// - Si le salon existe toujours → repopule les maps locales
///   (`VoiceOwnerMapKey`, `TextToVoiceMapKey`, `MembersToVoiceMapKey`)
///   pour que `check_and_delete_empty` puisse a nouveau les detecter
///   comme "temp".
/// - Si le salon n'existe plus (404 Discord) → appelle l'API pour le
///   marquer `closed` en BDD (nettoyage des fantomes laisses par un
///   crash ou un redemarrage).
async fn reconcile_voice_channels(ctx: &Context, ready: &Ready) {
    // Collecter toutes les refs dont on a besoin en un seul lock read.
    let (api, voice_owner, text_to_voice, members_to_voice) = {
        let data = ctx.data.read().await;
        let api = match crate::api_client::ApiClient::from_data(&data) {
            Some(a) => a,
            None => {
                warn!("reconcile_voice_channels: ApiClient absent, skip");
                return;
            }
        };
        let voice_owner: Option<Arc<DashMap<ChannelId, serenity::model::id::UserId>>> =
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

            // Verifier l'existence du salon cote Discord (404 = fantome).
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

            // Le salon existe → repopuler les maps locales.
            if let Some(ref map) = voice_owner {
                if let Ok(owner_id) = ch.owner_id.parse::<u64>() {
                    map.insert(voice_cid, serenity::model::id::UserId::new(owner_id));
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

    info!(
        reloaded,
        ghosts_closed,
        "reconcile_voice_channels termine"
    );
}

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!(bot = %ready.user.name, "Voice bot connecte");
        register_guilds(&ctx, &ready).await;

        // Reconciliation des salons vocaux : recharger la map locale
        // depuis la DB et fermer les lignes dont le salon Discord
        // n'existe plus (fantomes laisses par un crash / redemarrage).
        reconcile_voice_channels(&ctx, &ready).await;

        // Lancer le sweep AFK en arriere-plan
        crate::tasks::spawn_afk_sweep(ctx.clone());
    }

    async fn voice_state_update(&self, ctx: Context, old: Option<VoiceState>, new: VoiceState) {
        crate::handlers::voice::handle_voice_state_update(&ctx, &old, &new).await;
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        let guild_id_str = match &interaction {
            Interaction::Component(c) => c.guild_id.map(|g| g.to_string()),
            Interaction::Modal(m) => m.guild_id.map(|g| g.to_string()),
            _ => None,
        };
        if let Some(guild_id) = guild_id_str {
            let data = ctx.data.read().await;
            if let Some(api) = data.get::<ApiClientKey>() {
                let config = match api.get_guild_config(&guild_id).await {
                    Ok(cfg) => cfg,
                    Err(e) => {
                        tracing::warn!(error = %e, guild_id = %guild_id, "Echec chargement config guild");
                        std::collections::HashMap::new()
                    }
                };
                if !BaseApiClient::config_bool(&config, "enabled", true) {
                    return;
                }
            }
        }

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
        if let Some(guild_id) = msg.guild_id {
            let data = ctx.data.read().await;
            if let Some(api) = data.get::<ApiClientKey>() {
                let config = match api.get_guild_config(&guild_id.to_string()).await {
                    Ok(cfg) => cfg,
                    Err(e) => {
                        tracing::warn!(error = %e, guild_id = %guild_id, "Echec chargement config guild");
                        std::collections::HashMap::new()
                    }
                };
                if !BaseApiClient::config_bool(&config, "enabled", true) {
                    return;
                }
            }
        }
        crate::handlers::message::handle_message(&ctx, &msg).await;
    }
}
