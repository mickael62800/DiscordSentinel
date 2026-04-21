//! Creation et suppression des salons vocaux temporaires.
//!
//! Un salon temporaire = un vocal unique dont le panneau admin est poste
//! dans le chat integre du vocal (text-in-voice natif Discord). Plus de
//! categorie ni de salon texte separe. Pour les salons `game`, une file
//! d'attente (vocal secondaire) est creee en parallele.

use serenity::all::ButtonStyle;
use serenity::builder::{
    CreateActionRow, CreateButton, CreateChannel, CreateEmbed, CreateMessage, CreateSelectMenu,
    CreateSelectMenuKind,
};
use serenity::model::channel::{ChannelType, PermissionOverwrite, PermissionOverwriteType};
use serenity::model::id::{ChannelId, GuildId, UserId};
use serenity::model::Permissions;
use serenity::prelude::*;
use tracing::{error, info, warn};

use sentinel_shared::heartbeat::ApiClientKey;

use crate::modules::voice::api_client::{ApiClient, CreateVoiceChannelRequest};
use crate::modules::voice::embeds;
use crate::modules::voice::{CooldownTrackerKey, VoiceOwnerMapKey};

/// Cree un salon vocal temporaire (et sa queue si `kind == "game"`), deplace
/// l'utilisateur dedans et poste le panneau admin dans le chat integre du
/// vocal. `kind` = `"public"`, `"private"` ou `"game"`.
pub(super) async fn create_temp_channel(
    ctx: &Context,
    guild_id: GuildId,
    user_id: UserId,
    kind: &str,
) {
    // Cooldown check (anti-spam creation)
    {
        let data = ctx.data.read().await;
        if let Some(cooldowns) = data.get::<CooldownTrackerKey>() {
            if let Some(remaining) = cooldowns.check(user_id) {
                tracing::info!(user = %user_id, remaining = remaining, "Cooldown actif, creation ignoree");
                return;
            }
            cooldowns.set(user_id);
        }
    }

    let member = match guild_id.member(&ctx.http, user_id).await {
        Ok(m) => m,
        Err(_) => return,
    };
    let display_name = member.display_name().to_string();
    // Nom du vocal : prefix special pour les salons game
    let voice_name = if kind == "game" {
        format!("\u{1f3ae} {display_name}")
    } else {
        format!("Salon de {display_name}")
    };
    let everyone_role = guild_id.everyone_role();
    // user_limit par defaut : lu depuis le theme API si present, sinon 0.
    let default_user_limit: u32 = {
        let data = ctx.data.read().await;
        data.get::<crate::modules::voice::ThemeCacheKey>()
            .and_then(|themes| {
                themes.iter().find(|t| t.name == kind).and_then(|t| t.member_limit)
            })
            .unwrap_or(0) as u32
    };

    // Lire la categorie ancre depuis la config guild (pour le positionnement).
    let anchor_category_id: Option<u64> = {
        let data = ctx.data.read().await;
        if let Some(base) = data.get::<ApiClientKey>() {
            base.get_guild_config_for(&guild_id.to_string(), crate::modules::voice::MODULE_BOT_NAME)
                .await
                .ok()
                .and_then(|cfg| {
                    cfg.get("voice_anchor_category_id")
                        .and_then(|v| v.parse::<u64>().ok())
                        .filter(|id| *id > 0)
                })
        } else {
            None
        }
    };

    // 1. Creer le salon vocal. Si voice_anchor_category_id est configure,
    // on place le salon DANS cette categorie (Discord le met automatiquement
    // en bas de la categorie). Sinon, salon a la racine du serveur.
    let mut voice_builder = CreateChannel::new(&voice_name).kind(ChannelType::Voice);
    if default_user_limit > 0 {
        voice_builder = voice_builder.user_limit(default_user_limit);
    }
    if let Some(cat_id) = anchor_category_id {
        voice_builder = voice_builder.category(ChannelId::new(cat_id));
    }
    let voice_channel = match guild_id
        .create_channel(&ctx.http, voice_builder)
        .await
    {
        Ok(ch) => ch,
        Err(why) => {
            error!(error = %why, "Erreur creation salon vocal");
            return;
        }
    };
    let voice_channel_id = voice_channel.id;

    // Permissions owner sur le vocal (inclut SEND_MESSAGES pour le chat integre).
    let owner_perm = PermissionOverwrite {
        allow: Permissions::CONNECT
            | Permissions::VIEW_CHANNEL
            | Permissions::SPEAK
            | Permissions::SEND_MESSAGES
            | Permissions::MOVE_MEMBERS
            | Permissions::MUTE_MEMBERS
            | Permissions::DEAFEN_MEMBERS
            | Permissions::MANAGE_CHANNELS,
        deny: Permissions::empty(),
        kind: PermissionOverwriteType::Member(user_id),
    };
    if let Err(e) = voice_channel_id.create_permission(&ctx.http, owner_perm).await {
        tracing::warn!(error = %e, "failed to set owner permission on voice channel");
    }

    info!(channel = %voice_name, kind = %kind, "Salon vocal temporaire cree");

    // Stocker les mappings locaux AVANT le move.
    {
        let data = ctx.data.read().await;
        if let Some(map) = data.get::<VoiceOwnerMapKey>() {
            map.insert(voice_channel_id, user_id);
        }
    }

    // Deplacer l'utilisateur dans le vocal.
    if let Err(why) = guild_id.move_member(&ctx.http, user_id, voice_channel_id).await {
        warn!(error = %why, "Erreur deplacement membre");
    }

    // Pour les salons "game", creer automatiquement la file d'attente.
    let queue_channel_id: Option<ChannelId> = if kind == "game" {
        let queue_name = format!("File d'attente - {display_name}");
        let mut queue_builder = CreateChannel::new(&queue_name).kind(ChannelType::Voice);
        if let Some(cat_id) = anchor_category_id {
            queue_builder = queue_builder.category(ChannelId::new(cat_id));
        }
        match guild_id.create_channel(&ctx.http, queue_builder).await {
            Ok(qch) => {
                let queue_overwrite = PermissionOverwrite {
                    allow: Permissions::VIEW_CHANNEL | Permissions::CONNECT,
                    deny: Permissions::SPEAK,
                    kind: PermissionOverwriteType::Role(everyone_role),
                };
                if let Err(e) = qch.id.create_permission(&ctx.http, queue_overwrite).await {
                    warn!(error = %e, "failed to set queue channel permissions (game)");
                }
                let voice_overwrite = PermissionOverwrite {
                    allow: Permissions::empty(),
                    deny: Permissions::CONNECT,
                    kind: PermissionOverwriteType::Role(everyone_role),
                };
                if let Err(e) = voice_channel_id.create_permission(&ctx.http, voice_overwrite).await {
                    warn!(error = %e, "failed to lock game voice channel behind queue");
                }
                place_queue_above_voice(ctx, guild_id, qch.id, voice_channel_id).await;
                Some(qch.id)
            }
            Err(e) => {
                error!(error = %e, "Erreur creation queue channel (game)");
                None
            }
        }
    } else {
        None
    };

    // Envoyer le panneau de controle dans le chat integre du vocal
    // (prive + game uniquement ; les publics n'ont pas de panneau).
    if kind == "private" || kind == "game" {
        let queue_enabled_init = queue_channel_id.is_some();
        send_control_panel(ctx, voice_channel_id, false, queue_enabled_init, false, user_id.get()).await;
    }

    // Enregistrer via l'API (les champs texte/membres/categorie sont None
    // dans cette architecture simplifiee).
    {
        let data = ctx.data.read().await;
        if let Some(api) = ApiClient::from_data(&data) {
            let request = CreateVoiceChannelRequest {
                guild_id: guild_id.get().to_string(),
                owner_id: user_id.get().to_string(),
                owner_name: display_name.clone(),
                channel_id: voice_channel_id.get().to_string(),
                text_channel_id: None,
                members_channel_id: None,
                queue_channel_id: queue_channel_id.map(|id| id.get().to_string()),
                category_id: None,
                channel_name: voice_name.clone(),
                kind: kind.to_string(),
                visibility: "visible".to_string(),
                queue_enabled: queue_channel_id.is_some(),
            };

            if let Err(e) = api.create_channel(&request).await {
                warn!(error = %e, "Erreur API create_channel");
            }
        }
    }

    // Creer la carte de session dans le salon de logs.
    let creator_label = {
        let name = user_id
            .to_user(&ctx.http)
            .await
            .map(|u| u.name)
            .unwrap_or_else(|_| user_id.to_string());
        format!("{} (`{}`)", name, user_id)
    };
    embeds::create_session_card(ctx, voice_channel_id, &creator_label, kind).await;
}

/// Detecte si un salon temporaire est maintenant vide et, le cas echeant,
/// supprime le vocal (et la queue associee s'il y en a une).
///
/// Pour preserver la compat avec des salons pre-refacto encore en circulation,
/// on nettoie aussi les eventuels `text_channel_id` / `members_channel_id` /
/// `category_id` presents cote API mais nouvellement crees en vocal pur.
pub(super) async fn check_and_delete_empty(
    ctx: &Context,
    voice_channel_id: ChannelId,
    guild_id: GuildId,
) {
    let is_temp = {
        let data = ctx.data.read().await;
        data.get::<VoiceOwnerMapKey>()
            .map(|map| map.contains_key(&voice_channel_id))
            .unwrap_or(false)
    };

    if !is_temp {
        return;
    }

    let cleanup_delay = {
        let data = ctx.data.read().await;
        data.get::<crate::modules::voice::VoiceConfigKey>()
            .map(|c| c.empty_cleanup_delay_secs)
            .unwrap_or(2)
    };
    tokio::time::sleep(std::time::Duration::from_secs(cleanup_delay)).await;

    let is_empty = if let Some(guild) = ctx.cache.guild(guild_id) {
        guild
            .voice_states
            .values()
            .filter(|vs| vs.channel_id == Some(voice_channel_id))
            .count()
            == 0
    } else {
        false
    };

    if !is_empty {
        return;
    }

    let channel_name = embeds::get_channel_name(ctx, voice_channel_id).await;

    // Recupere les eventuels salons annexes legacy (queue + text + members + cat).
    let (queue_channel_id, legacy_text_id, legacy_members_id) = {
        let data = ctx.data.read().await;
        if let Some(api) = ApiClient::from_data(&data) {
            if let Ok(Some(ch)) = api.get_channel(&voice_channel_id.get().to_string()).await {
                let queue = ch
                    .queue_channel_id
                    .and_then(|id| id.parse::<u64>().ok())
                    .map(ChannelId::new);
                let text = ch
                    .text_channel_id
                    .and_then(|id| id.parse::<u64>().ok())
                    .map(ChannelId::new);
                let members = ch
                    .members_channel_id
                    .and_then(|id| id.parse::<u64>().ok())
                    .map(ChannelId::new);
                (queue, text, members)
            } else {
                (None, None, None)
            }
        } else {
            (None, None, None)
        }
    };

    // Supprimer via l'API.
    {
        let data = ctx.data.read().await;
        if let Some(api) = ApiClient::from_data(&data) {
            if let Err(e) = api
                .delete_channel(&voice_channel_id.get().to_string())
                .await
            {
                warn!(error = %e, "Erreur API delete_channel");
            }
        }
    }

    // Queue associee (game) : deconnecter les membres et supprimer.
    if let Some(queue_id) = queue_channel_id {
        let queue_members: Vec<_> = ctx
            .cache
            .guild(guild_id)
            .map(|guild| {
                guild
                    .voice_states
                    .values()
                    .filter(|vs| vs.channel_id == Some(queue_id))
                    .map(|vs| vs.user_id)
                    .collect()
            })
            .unwrap_or_default();

        for uid in queue_members {
            if let Err(e) = guild_id.disconnect_member(&ctx.http, uid).await {
                tracing::warn!(error = %e, user = %uid, "failed to disconnect member from queue");
            }
        }
        if let Err(e) = queue_id.delete(&ctx.http).await {
            tracing::warn!(error = %e, "failed to delete queue channel");
        }
        info!("Salon d'attente supprime: {queue_id}");
    }

    // Legacy text/members channels : si presents (salon cree avant la refonte),
    // on les supprime aussi pour que le menage reste complet.
    let legacy_category_id = if legacy_text_id.is_some() || legacy_members_id.is_some() {
        voice_channel_id
            .to_channel(&ctx.http)
            .await
            .ok()
            .and_then(|ch| ch.guild())
            .and_then(|gc| gc.parent_id)
    } else {
        None
    };

    if let Some(mid) = legacy_members_id {
        if let Err(e) = mid.delete(&ctx.http).await {
            warn!(error = %e, channel = %mid, "Erreur suppression panel membres legacy");
        }
    }

    if let Some(text_id) = legacy_text_id {
        if let Err(e) = text_id.delete(&ctx.http).await {
            warn!(error = %e, channel = %text_id, "Erreur suppression panel config legacy");
        }
        let data = ctx.data.read().await;
        if let Some(map) = data.get::<crate::modules::voice::TextToVoiceMapKey>() {
            map.remove(&text_id);
        }
    }

    if let Err(why) = voice_channel_id.delete(&ctx.http).await {
        error!(error = %why, "Erreur suppression salon vocal");
    } else {
        info!(channel = %channel_name, "Salon vocal supprime");
        embeds::session_closed(ctx, voice_channel_id, "session terminee").await;
    }

    if let Some(cat_id) = legacy_category_id {
        if let Err(e) = cat_id.delete(&ctx.http).await {
            warn!(error = %e, "Erreur suppression categorie legacy");
        }
    }

    {
        let data = ctx.data.read().await;
        if let Some(map) = data.get::<VoiceOwnerMapKey>() {
            map.remove(&voice_channel_id);
        }
    }
}

/// Place la file d'attente juste au-dessus du salon vocal principal.
pub async fn place_queue_above_voice(
    ctx: &Context,
    guild_id: GuildId,
    queue_channel_id: ChannelId,
    voice_channel_id: ChannelId,
) {
    let voice_pos = ctx
        .cache
        .guild(guild_id)
        .and_then(|g| g.channels.get(&voice_channel_id).map(|c| c.position));

    let voice_pos = match voice_pos {
        Some(p) => p as u64,
        None => return,
    };

    if let Err(e) = guild_id
        .reorder_channels(&ctx.http, [(queue_channel_id, voice_pos)])
        .await
    {
        warn!(
            error = %e,
            queue = %queue_channel_id,
            voice = %voice_channel_id,
            "reorder_channels echoue — la file d'attente reste en bas"
        );
    }
}

// ── Builders UI pour le panneau admin ──

async fn send_control_panel(
    ctx: &Context,
    text_channel_id: ChannelId,
    is_hidden: bool,
    queue_enabled: bool,
    locked: bool,
    owner_id: u64,
) {
    let visibility = if is_hidden { "Cache" } else { "Visible" };
    let queue_status = if queue_enabled { "Activee" } else { "Desactivee" };
    let lock_status = if locked { "Verrouille" } else { "Ouvert" };

    let embed = CreateEmbed::new()
        .title("Panneau de controle")
        .description(format!(
            "Salon prive de <@{owner_id}>\n\n\
            **Statut du salon :**\n\
            Visibilite : **{visibility}**\n\
            File d'attente : **{queue_status}**\n\
            Acces : **{lock_status}**\n\n\
            Utilise les **boutons** ci-dessous pour editer ton salon."
        ))
        .color(if locked {
            0xe67e22
        } else if is_hidden {
            0xe74c3c
        } else {
            0x2ecc71
        });

    let hide_label = if is_hidden { "Rendre visible" } else { "Cacher" };
    let queue_label = if queue_enabled {
        "Desactiver attente"
    } else {
        "File d'attente"
    };

    let mut row1 = vec![CreateButton::new("btn_hide")
        .label(hide_label)
        .style(if is_hidden {
            ButtonStyle::Success
        } else {
            ButtonStyle::Secondary
        })];

    if !is_hidden && !locked {
        row1.push(
            CreateButton::new("btn_queue")
                .label(queue_label)
                .style(if queue_enabled {
                    ButtonStyle::Success
                } else {
                    ButtonStyle::Secondary
                }),
        );
    }

    let lock_label = if locked { "Deverrouiller" } else { "Verrouiller" };

    let row2 = vec![
        CreateButton::new("btn_kick")
            .label("Kick")
            .style(ButtonStyle::Secondary),
        CreateButton::new("btn_ban")
            .label("Ban")
            .style(ButtonStyle::Danger),
        CreateButton::new("btn_lock")
            .label(lock_label)
            .style(if locked {
                ButtonStyle::Success
            } else {
                ButtonStyle::Secondary
            }),
        CreateButton::new("btn_limit")
            .label("Limite")
            .style(ButtonStyle::Secondary),
    ];

    let row3 = vec![
        CreateButton::new("btn_rename")
            .label("Renommer")
            .style(ButtonStyle::Secondary),
        CreateButton::new("btn_status")
            .label("Statut")
            .style(ButtonStyle::Secondary),
        CreateButton::new("btn_coadmin")
            .label("Co-admin")
            .style(ButtonStyle::Secondary),
        CreateButton::new("btn_transfer")
            .label("Transferer")
            .style(ButtonStyle::Secondary),
    ];

    let user_select = CreateSelectMenu::new(
        "select_invite",
        CreateSelectMenuKind::User {
            default_users: None,
        },
    )
    .placeholder("Inviter des membres dans le salon")
    .min_values(1)
    .max_values(25);

    let message = CreateMessage::new().embed(embed).components(vec![
        CreateActionRow::Buttons(row1),
        CreateActionRow::Buttons(row2),
        CreateActionRow::Buttons(row3),
        CreateActionRow::SelectMenu(user_select),
    ]);

    if let Err(why) = text_channel_id.send_message(&ctx.http, message).await {
        error!(error = %why, "Erreur envoi panneau de controle");
    }
}
