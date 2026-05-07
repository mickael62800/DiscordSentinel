//! Evenements voice state : join/leave/move, plus la logique de file d'attente.

use serenity::all::ButtonStyle;
use serenity::builder::{CreateActionRow, CreateButton, CreateMessage};
use serenity::model::id::{ChannelId, GuildId, UserId};
use serenity::model::voice::VoiceState;
use serenity::prelude::*;
use tracing::warn;

use crate::shared::api_client::BaseApiClient;
use crate::shared::heartbeat::ApiClientKey;

use crate::modules::voice::embeds;
use crate::modules::voice::{AfkTrackerKey, ConfigKey};

use super::channel_lifecycle::{check_and_delete_empty, create_temp_channel};

/// Point d'entree appele par le handler Discord sur chaque changement de voice state.
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

    let (public_creator_id, private_creator_id, game_creator_id) = {
        let data = ctx.data.read().await;
        let env_config = match data.get::<ConfigKey>() {
            Some(config) => (
                config.public_creator_channel_id,
                config.private_creator_channel_id,
            ),
            None => return,
        };

        if let Some(base) = data.get::<ApiClientKey>() {
            match base.get_guild_config_for(&guild_id.to_string(), crate::modules::voice::MODULE_BOT_NAME).await {
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

        if let Some(channel_id) = new_channel {
            embeds::session_member_joined(ctx, channel_id, &user_label).await;
        }

        if let Some(old_channel_id) = old_channel {
            embeds::session_member_left(ctx, old_channel_id, &user_label, "?").await;
        }
    }

    if let Some(channel_id) = new.channel_id {
        if channel_id == public_creator_id {
            create_temp_channel(ctx, guild_id, user_id, "public").await;
        } else if channel_id == private_creator_id {
            create_temp_channel(ctx, guild_id, user_id, "private").await;
        } else if game_creator_id == Some(channel_id) {
            create_temp_channel(ctx, guild_id, user_id, "game").await;
        } else {
            check_queue_join(ctx, guild_id, channel_id, user_id).await;
        }
    }

    // Auto-transfert d'ownership + delete-if-empty : UNIQUEMENT quand le user
    // quitte ou bouge de salon. Sans ce gate, un simple self_mute / self_deaf /
    // toggle video declenche un voice_state_update avec old_channel == new_channel
    // et faisait perdre le controle du salon a l'owner (regression).
    if let Some(old_channel_id) = should_run_leave_handlers(
        old_channel,
        new_channel,
        public_creator_id,
        private_creator_id,
    ) {
        maybe_auto_transfer_ownership(ctx, guild_id, old_channel_id, user_id).await;
        check_and_delete_empty(ctx, old_channel_id, guild_id).await;
    }

    // Tracking AFK
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

/// Si l'utilisateur qui vient de quitter etait l'owner d'un salon temporaire
/// et qu'au moins un autre membre reste, transfere automatiquement l'ownership.
async fn maybe_auto_transfer_ownership(
    ctx: &Context,
    guild_id: GuildId,
    voice_channel_id: ChannelId,
    leaving_user: UserId,
) {
    let is_owner = {
        let data = ctx.data.read().await;
        data.get::<crate::modules::voice::VoiceOwnerMapKey>()
            .and_then(|map| map.get(&voice_channel_id).map(|e| *e == leaving_user))
            .unwrap_or(false)
    };
    if !is_owner {
        return;
    }

    let old_perm = serenity::model::channel::PermissionOverwrite {
        allow: serenity::model::Permissions::CONNECT
            | serenity::model::Permissions::VIEW_CHANNEL
            | serenity::model::Permissions::SPEAK,
        deny: serenity::model::Permissions::empty(),
        kind: serenity::model::channel::PermissionOverwriteType::Member(leaving_user),
    };
    if let Err(e) = voice_channel_id.create_permission(&ctx.http, old_perm).await {
        tracing::warn!(error = %e, "failed to downgrade old owner permissions");
    }

    // Legacy : si ce vocal a encore un text_channel_id associe (cree avant la
    // refonte), on retire l'acces ; sinon on cible directement le vocal.
    let text_channel_id = {
        let data = ctx.data.read().await;
        data.get::<crate::modules::voice::TextToVoiceMapKey>().and_then(|map| {
            map.iter()
                .find(|entry| *entry.value() == voice_channel_id)
                .map(|entry| *entry.key())
        })
    };
    if let Some(tid) = text_channel_id {
        let deny_perm = serenity::model::channel::PermissionOverwrite {
            allow: serenity::model::Permissions::empty(),
            deny: serenity::model::Permissions::VIEW_CHANNEL,
            kind: serenity::model::channel::PermissionOverwriteType::Member(leaving_user),
        };
        if let Err(e) = tid.create_permission(&ctx.http, deny_perm).await {
            tracing::warn!(error = %e, "failed to revoke old owner admin panel access");
        }
    }

    let co_admin_candidate = find_co_admin_in_voice(ctx, guild_id, voice_channel_id, leaving_user).await;

    if let Some(new_owner) = co_admin_candidate {
        // On passe le chat integre du vocal comme cible pour le message de
        // notification (fallback sur l'eventuel legacy text_channel_id).
        let panel_target = text_channel_id.unwrap_or(voice_channel_id);
        do_direct_transfer(ctx, voice_channel_id, new_owner, Some(panel_target)).await;
        return;
    }

    // Prompt de reprise : on poste dans le chat integre du vocal.
    let prompt_target = text_channel_id.unwrap_or(voice_channel_id);

    let embed = serenity::builder::CreateEmbed::new()
        .title("\u{1f6a8} Le proprietaire a quitte le salon !")
        .description(
            "Le salon n'a plus d'admin.\n\
             Clique sur le bouton ci-dessous pour reprendre le controle."
        )
        .color(0xE67E22)
        .timestamp(serenity::model::Timestamp::now());

    let button = serenity::builder::CreateButton::new(
        format!("btn_claim_ownership_{}", voice_channel_id.get()),
    )
    .label("Reprendre le salon")
    .style(serenity::all::ButtonStyle::Success);

    let msg = serenity::builder::CreateMessage::new()
        .embed(embed)
        .components(vec![serenity::builder::CreateActionRow::Buttons(vec![button])]);

    if let Err(e) = prompt_target.send_message(&ctx.http, msg).await {
        tracing::warn!(error = %e, "failed to send claim ownership prompt");
    }

    tracing::info!(
        voice = %voice_channel_id,
        old_owner = %leaving_user,
        "Ownership en attente de candidature (owner a quitte)"
    );
}

async fn find_co_admin_in_voice(
    ctx: &Context,
    guild_id: GuildId,
    voice_channel_id: ChannelId,
    leaving_user: UserId,
) -> Option<UserId> {
    let co_admin_ids: Vec<u64> = {
        let data = ctx.data.read().await;
        let api = crate::modules::voice::api_client::ApiClient::from_data(&data)?;
        match api.get_channel_co_admins(&voice_channel_id.get().to_string()).await {
            Ok(ids) => ids.iter().filter_map(|s| s.parse().ok()).collect(),
            Err(e) => {
                tracing::warn!(error = %e, "failed to fetch co-admins for auto-transfer");
                return None;
            }
        }
    };
    if co_admin_ids.is_empty() {
        return None;
    }

    ctx.cache.guild(guild_id).and_then(|guild| {
        guild
            .voice_states
            .values()
            .filter(|vs| vs.channel_id == Some(voice_channel_id) && vs.user_id != leaving_user)
            .map(|vs| vs.user_id)
            .find(|uid| co_admin_ids.contains(&uid.get()))
    })
}

async fn do_direct_transfer(
    ctx: &Context,
    voice_channel_id: ChannelId,
    new_owner: UserId,
    text_channel_id: Option<ChannelId>,
) {
    let new_owner_name = new_owner
        .to_user(&ctx.http)
        .await
        .map(|u| u.name)
        .unwrap_or_else(|_| new_owner.to_string());

    {
        let data = ctx.data.read().await;
        if let Some(api) = crate::modules::voice::api_client::ApiClient::from_data(&data) {
            let req = crate::modules::voice::api_client::TransferOwnershipRequest {
                new_owner_id: new_owner.get().to_string(),
                new_owner_name: new_owner_name.clone(),
            };
            if let Err(e) = api
                .transfer_ownership(&voice_channel_id.get().to_string(), &req)
                .await
            {
                tracing::warn!(error = %e, "Erreur API transfer co-admin");
            }
        }
        if let Some(map) = data.get::<crate::modules::voice::VoiceOwnerMapKey>() {
            map.insert(voice_channel_id, new_owner);
        }
    }

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
        tracing::warn!(error = %e, "failed to grant co-admin owner permission");
    }

    if let Some(tid) = text_channel_id {
        let text_perm = serenity::model::channel::PermissionOverwrite {
            allow: serenity::model::Permissions::VIEW_CHANNEL
                | serenity::model::Permissions::SEND_MESSAGES
                | serenity::model::Permissions::READ_MESSAGE_HISTORY,
            deny: serenity::model::Permissions::empty(),
            kind: serenity::model::channel::PermissionOverwriteType::Member(new_owner),
        };
        if let Err(e) = tid.create_permission(&ctx.http, text_perm).await {
            tracing::warn!(error = %e, "failed to grant co-admin admin panel access");
        }

        let embed = serenity::builder::CreateEmbed::new()
            .title("\u{1f504} Co-admin promu proprietaire")
            .description(format!(
                "L'ancien proprietaire a quitte le vocal.\n\
                 <@{}> (co-admin) a ete automatiquement promu.",
                new_owner.get()
            ))
            .color(0x3498db)
            .timestamp(serenity::model::Timestamp::now());

        if let Err(e) = tid
            .send_message(&ctx.http, serenity::builder::CreateMessage::new().embed(embed))
            .await
        {
            tracing::warn!(error = %e, "failed to send co-admin promotion notification");
        }
    }

    tracing::info!(
        voice = %voice_channel_id,
        new_owner = %new_owner,
        "Co-admin promu automatiquement"
    );
}

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
    let Some(api) = crate::modules::voice::api_client::ApiClient::from_data(&data) else {
        return;
    };

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
        None => return,
    };

    let text_channel_id = parent
        .text_channel_id
        .as_ref()
        .and_then(|id| id.parse::<u64>().ok())
        .map(ChannelId::new);

    let owner_id = parent.owner_id.clone();

    drop(data);

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

/// Decide si les handlers "le user a quitte le salon" (auto-transfert
/// d'ownership + delete-if-empty) doivent etre executes.
///
/// Regle metier :
/// 1. Le user doit avoir un old_channel (sinon il vient juste de join).
/// 2. old_channel != new_channel (sinon ce n est pas un leave/move : c est
///    juste un self_mute / self_deaf / toggle video / streaming. Sans ce
///    gate, l owner perdait son ownership en se mutant — regression).
/// 3. old_channel ne doit PAS etre un creator channel (les creators sont
///    des "lobbies" qui ne portent pas d ownership).
///
/// Retourne `Some(old_channel_id)` si le bloc doit s executer, `None` sinon.
pub(super) fn should_run_leave_handlers(
    old_channel: Option<ChannelId>,
    new_channel: Option<ChannelId>,
    public_creator_id: ChannelId,
    private_creator_id: ChannelId,
) -> Option<ChannelId> {
    let old = old_channel?;
    if Some(old) == new_channel {
        return None;
    }
    if old == public_creator_id || old == private_creator_id {
        return None;
    }
    Some(old)
}

#[cfg(test)]
#[path = "tests/member_events.rs"]
mod tests;
