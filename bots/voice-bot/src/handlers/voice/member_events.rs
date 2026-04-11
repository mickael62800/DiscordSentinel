//! Événements voice state : join/leave/move, plus la logique de file d'attente.

use std::sync::Arc;

use serenity::all::ButtonStyle;
use serenity::builder::{CreateActionRow, CreateButton, CreateMessage};
use serenity::model::id::{ChannelId, GuildId, UserId};
use serenity::model::voice::VoiceState;
use serenity::prelude::*;
use tracing::warn;

use sentinel_shared::api_client::BaseApiClient;
use sentinel_shared::heartbeat::ApiClientKey;

use crate::embeds;
use crate::handler::{AfkTrackerKey, ConfigKey};

use super::channel_lifecycle::{check_and_delete_empty, create_temp_channel};
use super::channel_permissions::{grant_members_panel_access, revoke_members_panel_access};

/// Point d'entrée appelé par le handler Discord sur chaque changement de voice state.
///
/// Dispatche vers :
/// - création de salon temporaire si l'utilisateur rejoint un "creator channel",
/// - cleanup du salon si quelqu'un quitte un salon temporaire,
/// - gestion de la file d'attente pour les salons avec queue,
/// - tracking AFK (mute + deaf).
pub async fn handle_voice_state_update(
    ctx: &Context,
    old: &Option<VoiceState>,
    new: &VoiceState,
) {
    let guild_id = match new.guild_id.or(old.as_ref().and_then(|o| o.guild_id)) {
        Some(id) => id,
        None => return,
    };
    let user_id = new.user_id;

    // Charger les creator channel IDs : d'abord depuis l'API, fallback sur les env vars
    let (public_creator_id, private_creator_id) = {
        let data = ctx.data.read().await;
        let env_config = match data.get::<ConfigKey>() {
            Some(config) => (
                config.public_creator_channel_id,
                config.private_creator_channel_id,
            ),
            None => return,
        };

        // Tenter de charger depuis l'API
        if let Some(base) = data.get::<ApiClientKey>() {
            match base.get_guild_config(&guild_id.to_string()).await {
                Ok(config) => {
                    if !BaseApiClient::config_bool(&config, "enabled", true) {
                        return;
                    }
                    let public_id = config
                        .get("public_creator_channel_id")
                        .and_then(|v| v.parse::<u64>().ok())
                        .map(ChannelId::new)
                        .unwrap_or(env_config.0);
                    let private_id = config
                        .get("private_creator_channel_id")
                        .and_then(|v| v.parse::<u64>().ok())
                        .map(ChannelId::new)
                        .unwrap_or(env_config.1);
                    (public_id, private_id)
                }
                Err(e) => {
                    warn!(error = %e, "Config API indisponible, fallback sur env vars");
                    env_config
                }
            }
        } else {
            env_config
        }
    };

    // Determiner si c'est un vrai join/leave/move (pas un mute/unmute/camera)
    let old_channel = old.as_ref().and_then(|s| s.channel_id);
    let new_channel = new.channel_id;
    let channel_changed = old_channel != new_channel;

    if channel_changed {
        let user_label = {
            let name = user_id
                .to_user(&ctx.http)
                .await
                .map(|u| u.name)
                .unwrap_or_else(|_| user_id.to_string());
            format!("{} (`{}`)", name, user_id)
        };

        // Session card : membre rejoint un nouveau salon
        if let Some(channel_id) = new_channel {
            embeds::session_member_joined(ctx, channel_id, &user_label).await;
        }

        // Session card : membre quitte un salon
        if let Some(old_channel_id) = old_channel {
            embeds::session_member_left(ctx, old_channel_id, &user_label, "?").await;
        }
    }

    // Verifier si l'utilisateur a rejoint un salon createur
    if let Some(channel_id) = new.channel_id {
        if channel_id == public_creator_id {
            create_temp_channel(ctx, guild_id, user_id, "public").await;
        } else if channel_id == private_creator_id {
            create_temp_channel(ctx, guild_id, user_id, "private").await;
        } else {
            // Verifier file d'attente + donner acces panel membres
            check_queue_join(ctx, guild_id, channel_id, user_id).await;
            grant_members_panel_access(ctx, channel_id, user_id).await;
        }
    }

    // Quand quelqu'un quitte un salon vocal
    if let Some(old_state) = old {
        if let Some(old_channel_id) = old_state.channel_id {
            // Ne pas traiter les salons createurs
            if old_channel_id != public_creator_id && old_channel_id != private_creator_id {
                revoke_members_panel_access(ctx, old_channel_id, user_id).await;
                check_and_delete_empty(ctx, old_channel_id, guild_id).await;
            }
        }
    }

    // Tracking AFK : marquer ou retirer selon l'etat mute+sourd
    {
        let data = ctx.data.read().await;
        if let Some(afk_tracker) = data.get::<AfkTrackerKey>() {
            if new.channel_id.is_some() && new.self_mute && new.self_deaf {
                afk_tracker.mark_afk(user_id);
            } else {
                afk_tracker.clear(user_id);
            }
        }
    }
}

/// Gère l'arrivée d'un membre dans un "queue channel" (salon d'attente) :
/// notifie le owner par DM + poste des boutons Accepter/Refuser dans le panel admin.
async fn check_queue_join(
    ctx: &Context,
    guild_id: GuildId,
    channel_id: ChannelId,
    user_id: UserId,
) {
    if user_id.get() == 0 {
        return;
    }

    let data = ctx.data.read().await;
    let base = match data.get::<ApiClientKey>() {
        Some(b) => b,
        None => return,
    };

    // Chercher dans l'API quel voice channel a ce queue_channel_id
    let api = crate::api_client::ApiClient::new(Arc::clone(base), data.get::<sentinel_shared::grpc_client::GrpcClientKey>().expect("GrpcClientKey manquant").clone());
    let channels = match api.list_channels(&guild_id.to_string()).await {
        Ok(chs) => chs,
        Err(_) => return,
    };

    let channel_id_str = channel_id.get().to_string();
    let parent_channel = channels
        .iter()
        .find(|ch| ch.queue_channel_id.as_deref() == Some(&channel_id_str));

    let parent = match parent_channel {
        Some(ch) => ch,
        None => return, // Pas un canal d'attente connu
    };

    // Trouver le salon texte (admin panel) associe
    let text_channel_id = parent
        .text_channel_id
        .as_ref()
        .and_then(|id| id.parse::<u64>().ok())
        .map(ChannelId::new);

    let owner_id = parent.owner_id.clone();

    drop(data);

    // 1. Notifier dans le salon admin avec boutons Accepter/Refuser
    if let Some(text_id) = text_channel_id {
        let accept_id = format!("queue_accept_{}", user_id.get());
        let refuse_id = format!("queue_refuse_{}", user_id.get());

        let buttons = vec![
            CreateButton::new(&accept_id)
                .label("Accepter")
                .style(ButtonStyle::Success),
            CreateButton::new(&refuse_id)
                .label("Refuser")
                .style(ButtonStyle::Danger),
        ];

        let embed = serenity::builder::CreateEmbed::new()
            .title("\u{1f514} File d'attente")
            .description(format!(
                "<@{}> attend d'etre accepte dans votre salon vocal.",
                user_id
            ))
            .color(0xFFA500)
            .footer(serenity::builder::CreateEmbedFooter::new(
                "Cliquez pour accepter ou refuser",
            ))
            .timestamp(serenity::model::Timestamp::now());

        let message = CreateMessage::new()
            .embed(embed)
            .components(vec![CreateActionRow::Buttons(buttons)]);

        if let Err(why) = text_id.send_message(&ctx.http, message).await {
            warn!(error = %why, "Erreur notification file d'attente");
        }
    }

    // 2. Notifier l'owner par DM
    if let Ok(owner_uid) = owner_id.parse::<u64>() {
        let owner_user_id = UserId::new(owner_uid);
        if let Ok(user) = owner_user_id.to_user(&ctx.http).await {
            if let Ok(dm) = user.create_dm_channel(&ctx.http).await {
                let dm_embed = serenity::builder::CreateEmbed::new()
                    .title("\u{1f514} Quelqu'un attend dans votre salon !")
                    .description(format!(
                        "<@{}> est en file d'attente pour rejoindre votre salon vocal.\n\n\
                        Rendez-vous dans le panel admin de votre salon pour accepter ou refuser.",
                        user_id
                    ))
                    .color(0xFFA500)
                    .timestamp(serenity::model::Timestamp::now());

                if let Err(e) = dm
                    .send_message(&ctx.http, CreateMessage::new().embed(dm_embed))
                    .await
                {
                    tracing::warn!(error = %e, "failed to send queue DM notification to owner");
                }
            }
        }
    }
}
