use std::sync::Arc;
use serenity::all::ButtonStyle;
use serenity::builder::{
    CreateActionRow, CreateButton, CreateChannel, CreateEmbed, CreateMessage, CreateSelectMenu,
    CreateSelectMenuKind,
};
use serenity::model::channel::{ChannelType, PermissionOverwrite, PermissionOverwriteType};
use serenity::model::id::ChannelId;
use serenity::model::voice::VoiceState;
use serenity::model::Permissions;
use serenity::prelude::*;
use tracing::{error, info, warn};

use sentinel_shared::api_client::BaseApiClient;
use sentinel_shared::heartbeat::ApiClientKey;

use crate::api_client::{ApiClient, CreateVoiceChannelRequest};
use crate::handler::{
    AfkTrackerKey, ConfigKey, CooldownTrackerKey, MembersToVoiceMapKey,
    TextToVoiceMapKey, VoiceOwnerMapKey,
};
use crate::embeds;

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
            Some(config) => (config.public_creator_channel_id, config.private_creator_channel_id),
            None => return,
        };

        // Tenter de charger depuis l'API
        if let Some(base) = data.get::<ApiClientKey>() {
            match base.get_guild_config(&guild_id.to_string()).await {
                Ok(config) => {
                    if !BaseApiClient::config_bool(&config, "enabled", true) {
                        return;
                    }
                    let public_id = config.get("public_creator_channel_id")
                        .and_then(|v| v.parse::<u64>().ok())
                        .map(ChannelId::new)
                        .unwrap_or(env_config.0);
                    let private_id = config.get("private_creator_channel_id")
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
            let name = user_id.to_user(&ctx.http).await
                .map(|u| u.name).unwrap_or_else(|_| user_id.to_string());
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

/// Cree un salon temporaire complet (categorie + vocal + admin panel + membres panel)
/// et deplace l'utilisateur dedans. kind = "public" ou "private".
async fn create_temp_channel(
    ctx: &Context,
    guild_id: serenity::model::id::GuildId,
    user_id: serenity::model::id::UserId,
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
    let cat_name = format!("Salon de {display_name}");
    let everyone_role = guild_id.everyone_role();

    // Charger la position de base depuis la config guild
    let base_position: Option<u16> = {
        let data = ctx.data.read().await;
        if let Some(base) = data.get::<ApiClientKey>() {
            base.get_guild_config(&guild_id.to_string())
                .await
                .ok()
                .and_then(|cfg| cfg.get("voice_base_position").and_then(|v| v.parse().ok()))
        } else {
            None
        }
    };

    // 1. Creer la categorie (avec position si configuree)
    let mut create_cat = CreateChannel::new(&cat_name).kind(ChannelType::Category);
    if let Some(pos) = base_position {
        create_cat = create_cat.position(pos);
    }
    let cat = match guild_id
        .create_channel(&ctx.http, create_cat)
        .await
    {
        Ok(ch) => ch,
        Err(why) => {
            error!(error = %why, "Erreur creation categorie");
            return;
        }
    };

    // 2. Creer le salon vocal
    let voice_channel = match guild_id
        .create_channel(
            &ctx.http,
            CreateChannel::new("vocal")
                .kind(ChannelType::Voice)
                .category(cat.id),
        )
        .await
    {
        Ok(ch) => ch,
        Err(why) => {
            error!(error = %why, "Erreur creation salon vocal");
            if let Err(e) = cat.id.delete(&ctx.http).await {
                tracing::warn!(error = %e, "failed to delete category after voice channel creation error");
            }
            return;
        }
    };
    let voice_channel_id = voice_channel.id;

    // Permissions owner sur le vocal
    let owner_perm = PermissionOverwrite {
        allow: Permissions::CONNECT
            | Permissions::VIEW_CHANNEL
            | Permissions::SPEAK
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

    // 3. Panel admin config (prive seulement)
    let admin_channel_id = if kind == "private" {
        match guild_id
            .create_channel(
                &ctx.http,
                CreateChannel::new("config")
                    .kind(ChannelType::Text)
                    .category(cat.id)
                    .permissions(vec![
                        PermissionOverwrite {
                            allow: Permissions::empty(),
                            deny: Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES,
                            kind: PermissionOverwriteType::Role(everyone_role),
                        },
                        PermissionOverwrite {
                            allow: Permissions::VIEW_CHANNEL
                                | Permissions::SEND_MESSAGES
                                | Permissions::READ_MESSAGE_HISTORY,
                            deny: Permissions::empty(),
                            kind: PermissionOverwriteType::Member(user_id),
                        },
                    ]),
            )
            .await
        {
            Ok(ch) => Some(ch.id),
            Err(why) => {
                error!(error = %why, "Erreur creation panel admin");
                None
            }
        }
    } else {
        None
    };

    // 4. Panel membres (salon texte avec vote kick)
    let members_channel = match guild_id
        .create_channel(
            &ctx.http,
            CreateChannel::new("salon")
                .kind(ChannelType::Text)
                .category(cat.id)
                .permissions(vec![
                    PermissionOverwrite {
                        allow: Permissions::empty(),
                        deny: Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES,
                        kind: PermissionOverwriteType::Role(everyone_role),
                    },
                    PermissionOverwrite {
                        allow: Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES,
                        deny: Permissions::empty(),
                        kind: PermissionOverwriteType::Member(user_id),
                    },
                ]),
        )
        .await
    {
        Ok(ch) => ch,
        Err(why) => {
            error!(error = %why, "Erreur creation panel membres");
            if let Some(aid) = admin_channel_id {
                if let Err(e) = aid.delete(&ctx.http).await {
                    tracing::warn!(error = %e, "failed to delete admin channel during cleanup");
                }
            }
            if let Err(e) = voice_channel_id.delete(&ctx.http).await {
                tracing::warn!(error = %e, "failed to delete voice channel during cleanup");
            }
            if let Err(e) = cat.id.delete(&ctx.http).await {
                tracing::warn!(error = %e, "failed to delete category during cleanup");
            }
            return;
        }
    };

    info!(channel = %cat_name, kind = %kind, "Salon temporaire cree");

    // Stocker les mappings locaux AVANT le move
    {
        let data = ctx.data.read().await;
        if let Some(map) = data.get::<VoiceOwnerMapKey>() {
            map.insert(voice_channel_id, user_id);
        }
        if let Some(aid) = admin_channel_id {
            if let Some(map) = data.get::<TextToVoiceMapKey>() {
                map.insert(aid, voice_channel_id);
            }
        }
        if let Some(map) = data.get::<MembersToVoiceMapKey>() {
            map.insert(members_channel.id, voice_channel_id);
        }
    }

    // Deplacer l'utilisateur dans le vocal
    if let Err(why) = guild_id.move_member(&ctx.http, user_id, voice_channel_id).await {
        warn!(error = %why, "Erreur deplacement membre");
    }

    // Envoyer le panneau de controle (prive seulement)
    if let Some(aid) = admin_channel_id {
        send_control_panel(ctx, aid, false, false, false, user_id.get()).await;
    }

    // Envoyer le panel membres avec vote kick
    send_members_panel(ctx, members_channel.id).await;

    // Enregistrer via l'API
    {
        let data = ctx.data.read().await;
        if let Some(base) = data.get::<ApiClientKey>() {
            let api = ApiClient::new(base.clone());
            let request = CreateVoiceChannelRequest {
                guild_id: guild_id.get().to_string(),
                owner_id: user_id.get().to_string(),
                owner_name: display_name.clone(),
                channel_id: voice_channel_id.get().to_string(),
                text_channel_id: admin_channel_id.map(|id| id.get().to_string()),
                members_channel_id: Some(members_channel.id.get().to_string()),
                queue_channel_id: None,
                category_id: Some(cat.id.get().to_string()),
                channel_name: cat_name.clone(),
                kind: kind.to_string(),
                visibility: "visible".to_string(),
                queue_enabled: false,
            };

            if let Err(e) = api.create_channel(&request).await {
                warn!(error = %e, "Erreur API create_channel");
            }
        }
    }

    // Creer la carte de session dans le salon de logs
    let creator_label = {
        let name = user_id.to_user(&ctx.http).await
            .map(|u| u.name).unwrap_or_else(|_| user_id.to_string());
        format!("{} (`{}`)", name, user_id)
    };
    embeds::create_session_card(ctx, voice_channel_id, &creator_label, kind).await;
}

pub async fn send_control_panel(
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
    .max_values(10);

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

pub async fn send_members_panel(ctx: &Context, members_channel_id: ChannelId) {
    let embed = CreateEmbed::new()
        .title("Salon vocal")
        .description(
            "Bienvenue dans le salon !\n\n\
            Ce chat est reserve aux membres presents dans le vocal.\n\
            Quand tu quittes le vocal, tu perds l'acces a ce salon.\n\n\
            Si quelqu'un pose probleme et qu'il n'y a pas d'admin, utilise le **Vote Kick**.",
        )
        .color(0x3498db);

    let vote_select = CreateSelectMenu::new(
        "select_votekick",
        CreateSelectMenuKind::User {
            default_users: None,
        },
    )
    .placeholder("Vote Kick — Selectionner un membre a expulser")
    .min_values(1)
    .max_values(1);

    let message = CreateMessage::new()
        .embed(embed)
        .components(vec![CreateActionRow::SelectMenu(vote_select)]);

    if let Err(why) = members_channel_id.send_message(&ctx.http, message).await {
        error!(error = %why, "Erreur envoi panel membres");
    }
}

pub async fn grant_members_panel_access(
    ctx: &Context,
    voice_channel_id: ChannelId,
    user_id: serenity::model::id::UserId,
) {
    let members_channel_id = {
        let data = ctx.data.read().await;
        data.get::<MembersToVoiceMapKey>()
            .and_then(|map| {
                map.iter()
                    .find(|entry| *entry.value() == voice_channel_id)
                    .map(|entry| *entry.key())
            })
    };

    if let Some(mid) = members_channel_id {
        let perm = PermissionOverwrite {
            allow: Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES,
            deny: Permissions::empty(),
            kind: PermissionOverwriteType::Member(user_id),
        };
        if let Err(e) = mid.create_permission(&ctx.http, perm).await {
            tracing::warn!(error = %e, "failed to grant members panel access");
        }
    }
}

pub async fn revoke_members_panel_access(
    ctx: &Context,
    voice_channel_id: ChannelId,
    user_id: serenity::model::id::UserId,
) {
    // Ne pas retirer l'acces au owner ou co-admins
    let is_admin = {
        let data = ctx.data.read().await;
        data.get::<VoiceOwnerMapKey>()
            .and_then(|map| map.get(&voice_channel_id))
            .map(|owner| *owner == user_id)
            .unwrap_or(false)
    };

    if is_admin {
        return;
    }

    let members_channel_id = {
        let data = ctx.data.read().await;
        data.get::<MembersToVoiceMapKey>()
            .and_then(|map| {
                map.iter()
                    .find(|entry| *entry.value() == voice_channel_id)
                    .map(|entry| *entry.key())
            })
    };

    if let Some(mid) = members_channel_id {
        if let Err(e) = mid
            .delete_permission(&ctx.http, PermissionOverwriteType::Member(user_id))
            .await
        {
            tracing::warn!(error = %e, "failed to revoke members panel access");
        }
    }
}

async fn check_and_delete_empty(
    ctx: &Context,
    voice_channel_id: ChannelId,
    guild_id: serenity::model::id::GuildId,
) {
    // Verifier si c'est un salon temporaire (map locale, pre-remplie au demarrage)
    let is_temp = {
        let data = ctx.data.read().await;
        data.get::<VoiceOwnerMapKey>()
            .map(|map| map.contains_key(&voice_channel_id))
            .unwrap_or(false)
    };

    if !is_temp {
        return;
    }

    // Petit delai pour laisser le cache Discord se synchroniser
    // (evite de supprimer un salon pendant un move_member en cours)
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

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

    // Recuperer les infos du channel via l'API (pour le queue_channel_id)
    let queue_channel_id = {
        let data = ctx.data.read().await;
        if let Some(base) = data.get::<ApiClientKey>() {
            let api = ApiClient::new(base.clone());
            api.get_channel(&voice_channel_id.get().to_string())
                .await
                .ok()
                .flatten()
                .and_then(|ch| ch.queue_channel_id)
                .and_then(|id| id.parse::<u64>().ok())
                .map(ChannelId::new)
        } else {
            None
        }
    };

    // Supprimer via l'API
    {
        let data = ctx.data.read().await;
        if let Some(base) = data.get::<ApiClientKey>() {
            let api = ApiClient::new(base.clone());
            if let Err(e) = api
                .delete_channel(&voice_channel_id.get().to_string())
                .await
            {
                warn!(error = %e, "Erreur API delete_channel");
            }
        }
    }

    // Trouver les channels associes dans les maps locales
    let text_channel_id = {
        let data = ctx.data.read().await;
        data.get::<TextToVoiceMapKey>().and_then(|map| {
            map.iter()
                .find(|entry| *entry.value() == voice_channel_id)
                .map(|entry| *entry.key())
        })
    };

    let members_channel_id = {
        let data = ctx.data.read().await;
        data.get::<MembersToVoiceMapKey>().and_then(|map| {
            map.iter()
                .find(|entry| *entry.value() == voice_channel_id)
                .map(|entry| *entry.key())
        })
    };

    // Recuperer la categorie avant suppression
    let category_id = voice_channel_id
        .to_channel(&ctx.http)
        .await
        .ok()
        .and_then(|ch| ch.guild())
        .and_then(|gc| gc.parent_id);

    // Deconnecter tout le monde du salon d'attente et le supprimer
    if let Some(queue_id) = queue_channel_id {
        // Deconnecter les membres dans le queue
        let queue_members: Vec<_> = ctx.cache.guild(guild_id)
            .map(|guild| {
                guild.voice_states
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

    // Supprimer les channels texte AVANT le vocal et la categorie
    if let Some(mid) = members_channel_id {
        if let Err(e) = mid.delete(&ctx.http).await {
            warn!(error = %e, channel = %mid, "Erreur suppression panel membres");
        } else {
            info!(channel = %mid, "Panel membres supprime");
        }
        let data = ctx.data.read().await;
        if let Some(map) = data.get::<MembersToVoiceMapKey>() {
            map.remove(&mid);
        }
    }

    if let Some(text_id) = text_channel_id {
        if let Err(e) = text_id.delete(&ctx.http).await {
            warn!(error = %e, channel = %text_id, "Erreur suppression panel config");
        } else {
            info!(channel = %text_id, "Panel config supprime");
        }
        let data = ctx.data.read().await;
        if let Some(map) = data.get::<TextToVoiceMapKey>() {
            map.remove(&text_id);
        }
    }

    // Supprimer le salon vocal
    if let Err(why) = voice_channel_id.delete(&ctx.http).await {
        error!(error = %why, "Erreur suppression salon vocal");
    } else {
        info!(channel = %channel_name, "Salon vocal supprime");
        // Calculer la duree depuis la creation (approximation)
        embeds::session_closed(ctx, voice_channel_id, "session terminee").await;
    }

    // Supprimer la categorie en dernier
    if let Some(cat_id) = category_id {
        if let Err(e) = cat_id.delete(&ctx.http).await {
            warn!(error = %e, "Erreur suppression categorie");
        } else {
            info!(category = %cat_id, "Categorie supprimee");
        }
    }

    // Nettoyer les maps locales
    {
        let data = ctx.data.read().await;
        if let Some(map) = data.get::<VoiceOwnerMapKey>() {
            map.remove(&voice_channel_id);
        }
    }
}

async fn check_queue_join(
    ctx: &Context,
    guild_id: serenity::model::id::GuildId,
    channel_id: ChannelId,
    user_id: serenity::model::id::UserId,
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
    let api = crate::api_client::ApiClient::new(Arc::clone(base));
    let channels = match api.list_channels(&guild_id.to_string()).await {
        Ok(chs) => chs,
        Err(_) => return,
    };

    let channel_id_str = channel_id.get().to_string();
    let parent_channel = channels.iter().find(|ch| {
        ch.queue_channel_id.as_deref() == Some(&channel_id_str)
    });

    let parent = match parent_channel {
        Some(ch) => ch,
        None => return, // Pas un canal d'attente connu
    };

    // Trouver le salon texte (admin panel) associe
    let text_channel_id = parent.text_channel_id.as_ref()
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
            .footer(serenity::builder::CreateEmbedFooter::new("Cliquez pour accepter ou refuser"))
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
        let owner_user_id = serenity::model::id::UserId::new(owner_uid);
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

                if let Err(e) = dm.send_message(
                    &ctx.http,
                    CreateMessage::new().embed(dm_embed),
                ).await {
                    tracing::warn!(error = %e, "failed to send queue DM notification to owner");
                }
            }
        }
    }
}

