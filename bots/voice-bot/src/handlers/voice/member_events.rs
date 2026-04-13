//! Événements voice state : join/leave/move, plus la logique de file d'attente.

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
    // game_creator_channel_id est optionnel (None si non configure).
    let (public_creator_id, private_creator_id, game_creator_id) = {
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
                    let game_id = config
                        .get("game_creator_channel_id")
                        .and_then(|v| v.parse::<u64>().ok())
                        .filter(|id| *id > 0)
                        .map(ChannelId::new);
                    (public_id, private_id, game_id)
                }
                Err(e) => {
                    warn!(error = %e, "Config API indisponible, fallback sur env vars");
                    (env_config.0, env_config.1, None)
                }
            }
        } else {
            (env_config.0, env_config.1, None)
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
        } else if game_creator_id == Some(channel_id) {
            create_temp_channel(ctx, guild_id, user_id, "game").await;
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
                // Si l'owner vient de quitter un salon temporaire et qu'il
                // reste au moins une personne, transferer automatiquement
                // l'ownership pour ne pas laisser le salon sans admin.
                maybe_auto_transfer_ownership(ctx, guild_id, old_channel_id, user_id).await;
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

/// Si l'utilisateur qui vient de quitter `voice_channel_id` etait l'owner
/// d'un salon temporaire et qu'au moins un autre membre reste dans le vocal,
/// transfere automatiquement l'ownership au premier membre restant. Evite
/// qu'un salon devienne orphelin sans possibilite de reprise.
///
/// Note : on ne revoke PAS les permissions de l'ancien owner — s'il revient
/// dans le salon, il reste juste un membre sans privileges admin.
async fn maybe_auto_transfer_ownership(
    ctx: &Context,
    guild_id: GuildId,
    voice_channel_id: ChannelId,
    leaving_user: UserId,
) {
    // Est-ce un salon temporaire connu, et l'utilisateur est-il l'owner ?
    let is_owner = {
        let data = ctx.data.read().await;
        data.get::<crate::handler::VoiceOwnerMapKey>()
            .and_then(|map| map.get(&voice_channel_id).map(|e| *e == leaving_user))
            .unwrap_or(false)
    };
    if !is_owner {
        return;
    }

    // Trouver un remplacant : premier membre encore present dans le vocal
    // (hors l'utilisateur qui vient de partir et les bots).
    let candidate: Option<UserId> = ctx.cache.guild(guild_id).and_then(|guild| {
        guild
            .voice_states
            .values()
            .filter(|vs| vs.channel_id == Some(voice_channel_id) && vs.user_id != leaving_user)
            .map(|vs| vs.user_id)
            .find(|uid| {
                guild
                    .members
                    .get(uid)
                    .map(|m| !m.user.bot)
                    .unwrap_or(true)
            })
    });

    let new_owner = match candidate {
        Some(u) => u,
        None => return, // personne ne reste, check_and_delete_empty fera son travail
    };

    // Recuperer le nom du nouveau owner
    let new_owner_name = new_owner
        .to_user(&ctx.http)
        .await
        .map(|u| u.name)
        .unwrap_or_else(|_| new_owner.to_string());

    // Maj API + carte locale + permissions Discord
    {
        let data = ctx.data.read().await;
        if let Some(api) = crate::api_client::ApiClient::from_data(&data) {
            let req = crate::api_client::TransferOwnershipRequest {
                new_owner_id: new_owner.get().to_string(),
                new_owner_name: new_owner_name.clone(),
            };
            if let Err(e) = api
                .transfer_ownership(&voice_channel_id.get().to_string(), &req)
                .await
            {
                tracing::warn!(error = %e, "Erreur API transfer ownership automatique");
            }
        }
        if let Some(map) = data.get::<crate::handler::VoiceOwnerMapKey>() {
            map.insert(voice_channel_id, new_owner);
        }
    }

    // Donner au nouveau owner les permissions admin sur le vocal
    let owner_perm = serenity::model::channel::PermissionOverwrite {
        allow: serenity::model::Permissions::CONNECT
            | serenity::model::Permissions::VIEW_CHANNEL
            | serenity::model::Permissions::SPEAK
            | serenity::model::Permissions::MOVE_MEMBERS
            | serenity::model::Permissions::MUTE_MEMBERS
            | serenity::model::Permissions::DEAFEN_MEMBERS
            | serenity::model::Permissions::MANAGE_CHANNELS,
        deny: serenity::model::Permissions::empty(),
        kind: serenity::model::channel::PermissionOverwriteType::Member(new_owner),
    };
    if let Err(e) = voice_channel_id.create_permission(&ctx.http, owner_perm).await {
        tracing::warn!(error = %e, "failed to grant owner permission on auto-transfer");
    }

    // Donner acces au salon texte admin pour le nouveau owner
    let text_channel_id = {
        let data = ctx.data.read().await;
        data.get::<crate::handler::TextToVoiceMapKey>().and_then(|map| {
            map.iter()
                .find(|entry| *entry.value() == voice_channel_id)
                .map(|entry| *entry.key())
        })
    };
    if let Some(tid) = text_channel_id {
        let text_perm = serenity::model::channel::PermissionOverwrite {
            allow: serenity::model::Permissions::VIEW_CHANNEL
                | serenity::model::Permissions::SEND_MESSAGES
                | serenity::model::Permissions::READ_MESSAGE_HISTORY,
            deny: serenity::model::Permissions::empty(),
            kind: serenity::model::channel::PermissionOverwriteType::Member(new_owner),
        };
        if let Err(e) = tid.create_permission(&ctx.http, text_perm).await {
            tracing::warn!(error = %e, "failed to grant admin panel access to new owner");
        }

        // Notifier dans le salon admin
        let embed = serenity::builder::CreateEmbed::new()
            .title("\u{1f504} Transfert automatique")
            .description(format!(
                "L'ancien proprietaire a quitte le vocal.\n\
                 <@{}> est maintenant le proprietaire de ce salon.",
                new_owner.get()
            ))
            .color(0x3498db)
            .timestamp(serenity::model::Timestamp::now());

        if let Err(e) = tid
            .send_message(
                &ctx.http,
                serenity::builder::CreateMessage::new().embed(embed),
            )
            .await
        {
            tracing::warn!(error = %e, "failed to send auto-transfer notification");
        }
    }

    tracing::info!(
        voice = %voice_channel_id,
        old_owner = %leaving_user,
        new_owner = %new_owner,
        "Ownership transferee automatiquement (owner a quitte)"
    );
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
    let Some(api) = crate::api_client::ApiClient::from_data(&data) else {
        return;
    };

    // Chercher dans l'API quel voice channel a ce queue_channel_id
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
