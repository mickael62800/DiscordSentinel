//! Cycle de vie d'une table blackjack : création, invitation, join, fermeture.

use std::sync::Arc;

use serenity::all::{
    ButtonStyle, CreateActionRow, CreateButton, CreateChannel, CreateEmbed, CreateEmbedFooter,
    CreateInteractionResponse, CreateInteractionResponseMessage, CreateMessage,
    PermissionOverwrite, PermissionOverwriteType,
};
use serenity::model::application::ComponentInteraction;
use serenity::model::channel::ChannelType;
use serenity::model::id::RoleId;
use serenity::model::permissions::Permissions;
use serenity::prelude::*;
use tracing::{error, info, warn};

use super::{ChannelManagerKey, BET_PREFIX, CLOSE_TABLE_ID, INVITE_BUTTON_ID, JOIN_BUTTON_ID};

/// Clic sur "Jouer au Blackjack" du panel → crée un channel privé.
pub(super) async fn handle_panel_click(ctx: &Context, component: &ComponentInteraction) {
    let guild_id = match component.guild_id {
        Some(g) => g,
        None => return,
    };
    let user_id = component.user.id;

    // Verifier si deja une table ouverte
    {
        let data = ctx.data.read().await;
        if let Some(mgr) = data.get::<ChannelManagerKey>() {
            if let Some(table) = mgr.get(user_id) {
                let resp = CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content(format!("Tu as deja une table ouverte ! <#{}>", table.channel_id))
                        .ephemeral(true),
                );
                let _ = component.create_response(&ctx.http, resp).await;
                return;
            }
        }
    }

    // Repondre immediatement
    let resp = CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new()
            .content("\u{1f0cf} Ouverture de ta table privee...")
            .ephemeral(true),
    );
    if let Err(e) = component.create_response(&ctx.http, resp).await {
        warn!(error = %e, "Echec reponse panel click");
        return;
    }

    // Creer le channel prive
    let everyone_role = RoleId::new(guild_id.get());
    let channel_name = format!(
        "bj-{}",
        component
            .user
            .name
            .chars()
            .take(15)
            .collect::<String>()
            .to_lowercase()
    );

    let channel = match guild_id
        .create_channel(
            &ctx.http,
            CreateChannel::new(&channel_name)
                .kind(ChannelType::Text)
                .topic(format!("[blackjack:{}]", user_id))
                .permissions(vec![
                    PermissionOverwrite {
                        allow: Permissions::empty(),
                        deny: Permissions::VIEW_CHANNEL,
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
        Ok(ch) => ch,
        Err(e) => {
            error!(error = %e, "Echec creation channel blackjack");
            return;
        }
    };

    // Enregistrer dans le channel manager
    {
        let data = ctx.data.read().await;
        if let Some(mgr) = data.get::<ChannelManagerKey>() {
            mgr.register(user_id, channel.id, guild_id);
        }
    }

    // Creer la table en DB via l'API
    {
        let data = ctx.data.read().await;
        if let Some(api) = data.get::<crate::GameApiKey>() {
            if let Err(e) = api
                .create_table(
                    &guild_id.to_string(),
                    &channel.id.to_string(),
                    &user_id.to_string(),
                    &component.user.name,
                )
                .await
            {
                warn!(error = %e, "Echec creation table API (continue quand meme)");
            }
        }
    }

    // Accueil : mise + invite
    let embed = CreateEmbed::new()
        .title("\u{1f0cf} Table de Blackjack")
        .description(format!(
            "Bienvenue <@{}> !\n\n\
             \u{1f3b0} **Mise** — Choisis ton montant pour jouer\n\
             \u{1f465} **Inviter** — Ajoute des amis a la table\n\n\
             _Chaque joueur joue sa main contre le croupier.\n\
             La table ferme apres 30min d'inactivite._",
            user_id
        ))
        .color(0xF1C40F)
        .footer(CreateEmbedFooter::new(
            "Blackjack | Sentinel — Table multijoueur",
        ));

    let bet_buttons = vec![
        CreateButton::new(format!("{BET_PREFIX}50"))
            .label("50 \u{1fa99}")
            .style(ButtonStyle::Secondary),
        CreateButton::new(format!("{BET_PREFIX}100"))
            .label("100 \u{1fa99}")
            .style(ButtonStyle::Primary),
        CreateButton::new(format!("{BET_PREFIX}250"))
            .label("250 \u{1fa99}")
            .style(ButtonStyle::Primary),
        CreateButton::new(format!("{BET_PREFIX}500"))
            .label("500 \u{1fa99}")
            .style(ButtonStyle::Danger),
    ];
    let invite_button = CreateButton::new(INVITE_BUTTON_ID)
        .label("Inviter un joueur")
        .emoji(serenity::model::channel::ReactionType::Unicode(
            "\u{1f465}".into(),
        ))
        .style(ButtonStyle::Success);

    let row1 = CreateActionRow::Buttons(bet_buttons);
    let row2 = CreateActionRow::Buttons(vec![
        invite_button,
        CreateButton::new(CLOSE_TABLE_ID)
            .label("Fermer la table")
            .emoji(serenity::model::channel::ReactionType::Unicode(
                "\u{274c}".into(),
            ))
            .style(ButtonStyle::Danger),
    ]);

    let _ = channel
        .id
        .send_message(
            &ctx.http,
            CreateMessage::new().embed(embed).components(vec![row1, row2]),
        )
        .await;

    info!(
        user = %component.user.name,
        channel = %channel.id,
        "Table blackjack multijoueur ouverte"
    );
}

/// Invite : le créateur mentionne un joueur → le bot lui donne accès au channel.
pub(super) async fn handle_invite(ctx: &Context, component: &ComponentInteraction) {
    let _guild_id = match component.guild_id {
        Some(g) => g,
        None => return,
    };

    // Repondre avec un message demandant de mentionner quelqu'un
    let embed = CreateEmbed::new()
        .title("\u{1f465} Inviter un joueur")
        .description("Mentionne le joueur a inviter dans ce salon.\nExemple : `@MonAmi`")
        .color(0x3498db);

    let resp = CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new()
            .embed(embed)
            .ephemeral(true),
    );
    let _ = component.create_response(&ctx.http, resp).await;

    // Attendre un message avec une mention dans ce channel (timeout 30s)
    let channel_id = component.channel_id;
    let author_id = component.user.id;

    let ctx_clone = ctx.clone();
    tokio::spawn(async move {
        let timeout = tokio::time::Duration::from_secs(30);
        let start = tokio::time::Instant::now();

        loop {
            if start.elapsed() > timeout {
                break;
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

            // Lire les derniers messages
            let messages = match channel_id
                .messages(&ctx_clone.http, serenity::builder::GetMessages::new().limit(5))
                .await
            {
                Ok(m) => m,
                Err(_) => continue,
            };

            for msg in &messages {
                if msg.author.id != author_id || msg.mentions.is_empty() {
                    continue;
                }
                let now_ts = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as i64;
                if msg.timestamp.unix_timestamp() < (now_ts - 30) {
                    continue;
                }

                for mentioned in &msg.mentions {
                    if mentioned.bot || mentioned.id == author_id {
                        continue;
                    }

                    // Donner acces au channel
                    let overwrite = PermissionOverwrite {
                        allow: Permissions::VIEW_CHANNEL
                            | Permissions::SEND_MESSAGES
                            | Permissions::READ_MESSAGE_HISTORY,
                        deny: Permissions::empty(),
                        kind: PermissionOverwriteType::Member(mentioned.id),
                    };
                    let _ = channel_id
                        .create_permission(&ctx_clone.http, overwrite)
                        .await;

                    // Enregistrer dans l'API
                    {
                        let data = ctx_clone.data.read().await;
                        if let Some(api) = data.get::<crate::GameApiKey>() {
                            if let Ok(Some(table)) =
                                api.get_table_by_channel(&channel_id.to_string()).await
                            {
                                let _ = api
                                    .join_table(
                                        &table.id,
                                        &mentioned.id.to_string(),
                                        &mentioned.name,
                                    )
                                    .await;
                            }
                        }
                    }

                    // Notifier
                    let join_btn = CreateButton::new(JOIN_BUTTON_ID)
                        .label("Rejoindre la partie")
                        .emoji(serenity::model::channel::ReactionType::Unicode(
                            "\u{1f3b0}".into(),
                        ))
                        .style(ButtonStyle::Success);

                    let embed = CreateEmbed::new()
                        .description(format!(
                            "\u{1f465} <@{}> a ete invite a la table par <@{}> !\n\nClique sur **Rejoindre** pour miser et jouer.",
                            mentioned.id, author_id
                        ))
                        .color(0x2ecc71);

                    let _ = channel_id
                        .send_message(
                            &ctx_clone.http,
                            CreateMessage::new()
                                .embed(embed)
                                .components(vec![CreateActionRow::Buttons(vec![join_btn])]),
                        )
                        .await;

                    info!(
                        invited = %mentioned.name,
                        by = %author_id,
                        "Joueur invite a la table blackjack"
                    );
                }

                // Supprimer le message de mention
                let _ = msg.delete(&ctx_clone.http).await;
                return;
            }
        }
    });
}

/// Un joueur invité clique "Rejoindre" → affiche les boutons de mise.
pub(super) async fn handle_join(ctx: &Context, component: &ComponentInteraction) {
    // Touch activity
    {
        let data = ctx.data.read().await;
        if let Some(mgr) = data.get::<ChannelManagerKey>() {
            // On ne register pas le joueur invite dans le channel manager (seul le owner y est)
            // Mais on touch le timer du owner pour eviter le AFK
            if let Some((owner_id, _)) = mgr.find_by_channel(component.channel_id) {
                mgr.touch(owner_id);
            }
        }
    }

    let embed = CreateEmbed::new()
        .title(format!(
            "\u{1f0cf} {} rejoint la table !",
            component.user.name
        ))
        .description("Choisis ta mise pour cette manche :")
        .color(0xF1C40F);

    let buttons = vec![
        CreateButton::new(format!("{BET_PREFIX}50"))
            .label("50 \u{1fa99}")
            .style(ButtonStyle::Secondary),
        CreateButton::new(format!("{BET_PREFIX}100"))
            .label("100 \u{1fa99}")
            .style(ButtonStyle::Primary),
        CreateButton::new(format!("{BET_PREFIX}250"))
            .label("250 \u{1fa99}")
            .style(ButtonStyle::Primary),
        CreateButton::new(format!("{BET_PREFIX}500"))
            .label("500 \u{1fa99}")
            .style(ButtonStyle::Danger),
    ];

    component
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .embed(embed)
                    .components(vec![CreateActionRow::Buttons(buttons)])
                    .ephemeral(true),
            ),
        )
        .await
        .ok();
}

/// Ferme la table (supprime le channel).
pub(super) async fn handle_close_table(ctx: &Context, component: &ComponentInteraction) {
    let resp = CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new()
            .content("\u{1f44b} Table fermee ! A bientot.")
            .ephemeral(true),
    );
    let _ = component.create_response(&ctx.http, resp).await;

    let data = ctx.data.read().await;
    let mgr = match data.get::<ChannelManagerKey>() {
        Some(m) => Arc::clone(m),
        None => return,
    };
    let api = data.get::<crate::GameApiKey>().cloned();
    drop(data);

    mgr.remove(component.user.id);

    // Marquer la table comme fermee en DB (sinon la row reste 'open' orpheline).
    if let Some(api) = api {
        match api.get_table_by_channel(&component.channel_id.to_string()).await {
            Ok(Some(table)) => {
                if let Err(e) = api.close_table(&table.id).await {
                    warn!(error = %e, table_id = %table.id, "Echec close_table API");
                }
            }
            Ok(None) => {}
            Err(e) => warn!(error = %e, "Echec lookup table by channel"),
        }
    }

    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    if let Err(e) = component.channel_id.delete(&ctx.http).await {
        warn!(error = %e, "Echec suppression channel blackjack");
    } else {
        info!(user = %component.user.name, "Table blackjack fermee par le joueur");
    }
}
