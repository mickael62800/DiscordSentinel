//! Création et suppression des salons vocaux temporaires.
//!
//! Un salon temporaire = catégorie + salon vocal + panel admin (si privé)
//! + panel membres. La création est atomique "best effort" (rollback manuel
//! si une étape échoue). La suppression détecte l'état "vide + temp" avant
//! de nettoyer toute la famille de salons.

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

use crate::api_client::{ApiClient, CreateVoiceChannelRequest};
use crate::embeds;
use crate::handler::{
    CooldownTrackerKey, MembersToVoiceMapKey, TextToVoiceMapKey, VoiceOwnerMapKey,
};

/// Crée un salon temporaire complet (catégorie + vocal + admin panel + panel membres)
/// et déplace l'utilisateur dedans. `kind` = `"public"` ou `"private"`.
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
    // Nom de la categorie : prefix special pour les salons game
    let cat_name = if kind == "game" {
        format!("🎮 {display_name}")
    } else {
        format!("Salon de {display_name}")
    };
    let everyone_role = guild_id.everyone_role();
    // user_limit par defaut : lu depuis le theme API si present, sinon 0.
    let default_user_limit: u32 = {
        let data = ctx.data.read().await;
        data.get::<crate::handler::ThemeCacheKey>()
            .and_then(|themes| {
                themes.iter().find(|t| t.name == kind).and_then(|t| t.member_limit)
            })
            .unwrap_or(0) as u32
    };

    // Lire la categorie ancre depuis la config guild. Si configuree, on
    // lira sa position ACTUELLE depuis le cache Discord pour placer le
    // nouveau salon juste en dessous (position + 1).
    let anchor_category_id: Option<u64> = {
        let data = ctx.data.read().await;
        if let Some(base) = data.get::<ApiClientKey>() {
            base.get_guild_config(&guild_id.to_string())
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

    // Lookup de la position actuelle de l'ancre dans le cache Serenity.
    // Si absente du cache ou non configuree, on laisse Discord placer le
    // salon en bas (comportement par defaut).
    let target_position: Option<u16> = anchor_category_id.and_then(|anchor_id| {
        let anchor_channel = ChannelId::new(anchor_id);
        ctx.cache.guild(guild_id).and_then(|g| {
            g.channels
                .get(&anchor_channel)
                .map(|ch| ch.position.saturating_add(1))
        })
    });

    // 1. Creer la categorie (sans position — Discord accepte .position() mais
    //    ne decale pas les autres categories existantes, ce qui cree une
    //    collision visuelle differente selon le client : admin voit bien,
    //    user classique voit la nouvelle cat en fin de liste. Fix : on cree
    //    d'abord puis on force la position via reorder_channels (endpoint
    //    Discord PATCH /guilds/{id}/channels qui fait le shift atomique).
    let create_cat = CreateChannel::new(&cat_name).kind(ChannelType::Category);
    let cat = match guild_id.create_channel(&ctx.http, create_cat).await {
        Ok(ch) => ch,
        Err(why) => {
            error!(error = %why, "Erreur creation categorie");
            return;
        }
    };

    // Si une position cible est configuree (ancre), forcer le reorder via
    // l'endpoint dedie de Discord qui gere le shift des autres channels.
    if let Some(pos) = target_position {
        if let Err(e) = guild_id
            .reorder_channels(&ctx.http, [(cat.id, pos as u64)])
            .await
        {
            warn!(
                error = %e,
                cat_id = %cat.id,
                target_pos = pos,
                "reorder_channels echoue — la nouvelle categorie sera en bas pour certains clients"
            );
        }
    }

    // 2. Creer le salon vocal
    let mut voice_builder = CreateChannel::new("vocal")
        .kind(ChannelType::Voice)
        .category(cat.id);
    if default_user_limit > 0 {
        voice_builder = voice_builder.user_limit(default_user_limit);
    }
    let voice_channel = match guild_id
        .create_channel(&ctx.http, voice_builder)
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

    // 3. Panel admin config (prive + game — ces 2 types ont un panel owner)
    let admin_channel_id = if kind == "private" || kind == "game" {
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

    // 5. Pour les salons "game", creer automatiquement la file d'attente
    //    et locker le vocal principal (deny CONNECT @everyone). Les user
    //    overrides ALLOW sur l'owner priment donc il peut rejoindre.
    //    Tout autre user doit passer par la queue + etre accepte.
    let queue_channel_id: Option<ChannelId> = if kind == "game" {
        let queue_name = format!("File d'attente - {display_name}");
        let queue_builder = CreateChannel::new(&queue_name)
            .kind(ChannelType::Voice)
            .category(cat.id);
        match guild_id.create_channel(&ctx.http, queue_builder).await {
            Ok(qch) => {
                // Permissions queue : everyone peut join mais pas speak
                let queue_overwrite = PermissionOverwrite {
                    allow: Permissions::VIEW_CHANNEL | Permissions::CONNECT,
                    deny: Permissions::SPEAK,
                    kind: PermissionOverwriteType::Role(everyone_role),
                };
                if let Err(e) = qch.id.create_permission(&ctx.http, queue_overwrite).await {
                    warn!(error = %e, "failed to set queue channel permissions (game)");
                }
                // Deny CONNECT sur le vocal principal pour @everyone
                let voice_overwrite = PermissionOverwrite {
                    allow: Permissions::empty(),
                    deny: Permissions::CONNECT,
                    kind: PermissionOverwriteType::Role(everyone_role),
                };
                if let Err(e) = voice_channel_id.create_permission(&ctx.http, voice_overwrite).await {
                    warn!(error = %e, "failed to lock game voice channel behind queue");
                }
                // Placer la file d'attente AU-DESSUS du vocal principal pour
                // le rendre plus visible (reorder_channels pousse le vocal
                // d'un cran vers le bas dans la categorie).
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

    // Envoyer le panneau de controle (prive + game)
    if let Some(aid) = admin_channel_id {
        let queue_enabled_init = queue_channel_id.is_some();
        send_control_panel(ctx, aid, false, false, queue_enabled_init, user_id.get()).await;
    }

    // Envoyer le panel membres avec vote kick
    send_members_panel(ctx, members_channel.id).await;

    // Enregistrer via l'API
    {
        let data = ctx.data.read().await;
        if let Some(api) = ApiClient::from_data(&data) {
            let request = CreateVoiceChannelRequest {
                guild_id: guild_id.get().to_string(),
                owner_id: user_id.get().to_string(),
                owner_name: display_name.clone(),
                channel_id: voice_channel_id.get().to_string(),
                text_channel_id: admin_channel_id.map(|id| id.get().to_string()),
                members_channel_id: Some(members_channel.id.get().to_string()),
                queue_channel_id: queue_channel_id.map(|id| id.get().to_string()),
                category_id: Some(cat.id.get().to_string()),
                channel_name: cat_name.clone(),
                kind: kind.to_string(),
                visibility: "visible".to_string(),
                queue_enabled: queue_channel_id.is_some(),
            };

            if let Err(e) = api.create_channel(&request).await {
                warn!(error = %e, "Erreur API create_channel");
            }
        }
    }

    // Creer la carte de session dans le salon de logs
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

/// Détecte si un salon temporaire est maintenant vide et, le cas échéant,
/// supprime toute la famille (catégorie + vocal + panels + queue).
pub(super) async fn check_and_delete_empty(
    ctx: &Context,
    voice_channel_id: ChannelId,
    guild_id: GuildId,
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

    // Delai anti-race configurable : laisse le cache Discord se synchroniser
    // (evite de supprimer un salon pendant un move_member en cours).
    let cleanup_delay = {
        let data = ctx.data.read().await;
        data.get::<crate::handler::VoiceConfigKey>()
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

    // Recuperer les infos du channel via l'API (pour le queue_channel_id)
    let queue_channel_id = {
        let data = ctx.data.read().await;
        if let Some(api) = ApiClient::from_data(&data) {
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
        if let Some(api) = ApiClient::from_data(&data) {
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

/// Place la file d'attente juste au-dessus du salon vocal principal dans la
/// categorie. Utilise `reorder_channels` qui gere le shift atomique des
/// autres channels. Si le lookup echoue, le salon reste en bas (comportement
/// par defaut de Discord) — l'echec est non fatal.
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

    // On donne au queue la position actuelle du voice ; Discord decale le
    // voice (et les suivants) d'un cran vers le bas automatiquement.
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

// ── Builders UI pour les panels (utilisés uniquement lors de la création) ──

/// Envoie le panel admin (boutons de contrôle du salon) dans le salon texte privé.
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

/// Envoie le panel membres (avec bouton vote kick) dans le salon texte partagé.
async fn send_members_panel(ctx: &Context, members_channel_id: ChannelId) {
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
