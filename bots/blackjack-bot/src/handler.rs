use std::sync::Arc;

use serenity::all::{
    ButtonStyle, CreateActionRow, CreateButton, CreateChannel, CreateEmbed, CreateEmbedFooter,
    CreateInteractionResponse, CreateInteractionResponseMessage, CreateMessage,
    PermissionOverwrite, PermissionOverwriteType,
};
use serenity::async_trait;
use serenity::model::application::Interaction;
use serenity::model::channel::ChannelType;
use serenity::model::gateway::Ready;
use serenity::model::id::RoleId;
use serenity::model::permissions::Permissions;
use serenity::prelude::*;
use tracing::{error, info, warn};

use sentinel_shared::heartbeat::{ApiClientKey, register_guilds};

use crate::channel_manager::ChannelManager;
use crate::commands;

pub struct ChannelManagerKey;
impl TypeMapKey for ChannelManagerKey {
    type Value = Arc<ChannelManager>;
}

/// Timeout AFK en secondes (30 minutes).
const AFK_TIMEOUT_SECS: u64 = 1800;

/// Custom IDs pour les boutons de mise.
const BET_PREFIX: &str = "bj_bet:";
const CLOSE_TABLE_ID: &str = "bj_close_table";
const INVITE_BUTTON_ID: &str = "bj_invite";
const JOIN_BUTTON_ID: &str = "bj_join_table";

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!(bot = %ready.user.name, "Blackjack bot connecte");

        register_guilds(&ctx, &ready).await;

        if let Err(e) = serenity::model::application::Command::set_global_commands(
            &ctx.http,
            commands::all(),
        )
        .await
        {
            error!(error = %e, "Erreur enregistrement commandes");
        } else {
            info!("Slash commands enregistrees : blackjack-setup");
        }

        // Background task : cleanup des tables AFK toutes les 60s
        let ctx_clone = ctx.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;

                let data = ctx_clone.data.read().await;
                let mgr = match data.get::<ChannelManagerKey>() {
                    Some(m) => Arc::clone(m),
                    None => continue,
                };
                drop(data);

                let afk = mgr.afk_channels(AFK_TIMEOUT_SECS);
                for (user_id, table) in afk {
                    let embed = CreateEmbed::new()
                        .title("\u{23f0} Table fermee — Inactivite")
                        .description("Cette table de blackjack a ete fermee apres 30 minutes d'inactivite.")
                        .color(0x95A5A6);
                    let _ = table.channel_id
                        .send_message(&ctx_clone.http, CreateMessage::new().embed(embed))
                        .await;

                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

                    if let Err(e) = table.channel_id.delete(&ctx_clone.http).await {
                        warn!(error = %e, "Echec suppression channel AFK blackjack");
                    } else {
                        info!(user = %user_id, "Table blackjack AFK supprimee");
                    }

                    mgr.remove(user_id);
                }
            }
        });
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        match interaction {
            Interaction::Command(command) => {
                if let Some(guild_id) = command.guild_id {
                    let data = ctx.data.read().await;
                    if let Some(api) = data.get::<ApiClientKey>() {
                        if !sentinel_shared::discord_helpers::is_bot_enabled(api, &guild_id.to_string()).await {
                            return;
                        }
                    }
                }

                match command.data.name.as_str() {
                    "blackjack-setup" => commands::setup::handle(&ctx, &command).await,
                    _ => {}
                }
            }
            Interaction::Component(component) => {
                let custom_id = component.data.custom_id.clone();

                if custom_id == commands::setup::PANEL_BUTTON_ID {
                    handle_panel_click(&ctx, &component).await;
                } else if custom_id.starts_with(BET_PREFIX) {
                    handle_bet_select(&ctx, &component).await;
                } else if custom_id == CLOSE_TABLE_ID {
                    handle_close_table(&ctx, &component).await;
                } else if custom_id == INVITE_BUTTON_ID {
                    handle_invite(&ctx, &component).await;
                } else if custom_id == JOIN_BUTTON_ID {
                    handle_join(&ctx, &component).await;
                } else if custom_id.starts_with("bj_hit:")
                    || custom_id.starts_with("bj_stand:")
                    || custom_id.starts_with("bj_double:")
                {
                    // Touch activity
                    {
                        let data = ctx.data.read().await;
                        if let Some(mgr) = data.get::<ChannelManagerKey>() {
                            mgr.touch(component.user.id);
                        }
                    }
                    commands::blackjack::handle_component(&ctx, &component).await;
                    check_game_over(&ctx, &component).await;
                }
            }
            _ => {}
        }
    }
}

/// Clic sur "Jouer au Blackjack" du panel → cree un channel prive.
async fn handle_panel_click(ctx: &Context, component: &serenity::model::application::ComponentInteraction) {
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
    let channel_name = format!("bj-{}", component.user.name.chars().take(15).collect::<String>().to_lowercase());

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
                        allow: Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES | Permissions::READ_MESSAGE_HISTORY,
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
            if let Err(e) = api.create_table(
                &guild_id.to_string(), &channel.id.to_string(),
                &user_id.to_string(), &component.user.name,
            ).await {
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
        .footer(CreateEmbedFooter::new("Blackjack | Sentinel — Table multijoueur"));

    let bet_buttons = vec![
        CreateButton::new(format!("{BET_PREFIX}50")).label("50 \u{1fa99}").style(ButtonStyle::Secondary),
        CreateButton::new(format!("{BET_PREFIX}100")).label("100 \u{1fa99}").style(ButtonStyle::Primary),
        CreateButton::new(format!("{BET_PREFIX}250")).label("250 \u{1fa99}").style(ButtonStyle::Primary),
        CreateButton::new(format!("{BET_PREFIX}500")).label("500 \u{1fa99}").style(ButtonStyle::Danger),
    ];
    let invite_button = CreateButton::new(INVITE_BUTTON_ID)
        .label("Inviter un joueur")
        .emoji(serenity::model::channel::ReactionType::Unicode("\u{1f465}".into()))
        .style(ButtonStyle::Success);

    let row1 = CreateActionRow::Buttons(bet_buttons);
    let row2 = CreateActionRow::Buttons(vec![invite_button,
        CreateButton::new(CLOSE_TABLE_ID)
            .label("Fermer la table")
            .emoji(serenity::model::channel::ReactionType::Unicode("\u{274c}".into()))
            .style(ButtonStyle::Danger),
    ]);

    let _ = channel.id.send_message(&ctx.http, CreateMessage::new().embed(embed).components(vec![row1, row2])).await;

    info!(user = %component.user.name, channel = %channel.id, "Table blackjack multijoueur ouverte");
}

/// Selection de mise → demarre la partie.
async fn handle_bet_select(ctx: &Context, component: &serenity::model::application::ComponentInteraction) {
    let bet: i64 = match component.data.custom_id.strip_prefix(BET_PREFIX).and_then(|b| b.parse().ok()) {
        Some(b) => b,
        None => return,
    };

    let guild_id = match component.guild_id {
        Some(g) => g.to_string(),
        None => return,
    };

    // Touch
    {
        let data = ctx.data.read().await;
        if let Some(mgr) = data.get::<ChannelManagerKey>() {
            mgr.touch(component.user.id);
        }
    }

    let data = ctx.data.read().await;
    let api = match data.get::<crate::GameApiKey>() {
        Some(a) => a,
        None => return,
    };

    let game = match api.start_game(&guild_id, &component.user.id.to_string(), &component.user.name, bet).await {
        Ok(g) => g,
        Err(e) => {
            let resp = CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new().content(format!("Erreur : {e}")).ephemeral(true),
            );
            let _ = component.create_response(&ctx.http, resp).await;
            return;
        }
    };

    // Stocker game_id
    if let Some(mgr) = data.get::<ChannelManagerKey>() {
        mgr.set_game_id(component.user.id, game.id.clone());
    }
    drop(data);

    let embed = commands::blackjack::build_game_embed(&game);
    let components = if commands::blackjack::is_game_over(&game.status) {
        vec![]
    } else {
        commands::blackjack::build_buttons(&game)
    };

    component
        .create_response(
            &ctx.http,
            CreateInteractionResponse::UpdateMessage(
                CreateInteractionResponseMessage::new().embed(embed).components(components),
            ),
        )
        .await
        .ok();

    // Si blackjack naturel → proposer de rejouer
    if commands::blackjack::is_game_over(&game.status) {
        send_replay_buttons(ctx, component.channel_id).await;
    }
}

/// Verifie si la partie est terminee apres un hit/stand/double.
async fn check_game_over(ctx: &Context, component: &serenity::model::application::ComponentInteraction) {
    let guild_id = match component.guild_id {
        Some(g) => g.to_string(),
        None => return,
    };

    let is_over = {
        let data = ctx.data.read().await;
        let api = match data.get::<crate::GameApiKey>() {
            Some(a) => a,
            None => return,
        };
        match api.get_active(&guild_id, &component.user.id.to_string()).await {
            Ok(Some(game)) => commands::blackjack::is_game_over(&game.status),
            Ok(None) => true, // Pas de partie active = terminee
            Err(_) => false,
        }
    };

    if is_over {
        send_replay_buttons(ctx, component.channel_id).await;
    }
}

/// Envoie les boutons "Rejouer" ou "Fermer la table".
async fn send_replay_buttons(ctx: &Context, channel_id: serenity::model::id::ChannelId) {
    let embed = CreateEmbed::new()
        .description("\u{1f503} **Encore une partie ?**")
        .color(0x3498db);

    let buttons = vec![
        CreateButton::new(format!("{BET_PREFIX}50")).label("50").style(ButtonStyle::Secondary),
        CreateButton::new(format!("{BET_PREFIX}100")).label("100").style(ButtonStyle::Primary),
        CreateButton::new(format!("{BET_PREFIX}250")).label("250").style(ButtonStyle::Primary),
        CreateButton::new(CLOSE_TABLE_ID).label("Fermer la table").emoji(serenity::model::channel::ReactionType::Unicode("\u{274c}".into())).style(ButtonStyle::Danger),
    ];
    let row = CreateActionRow::Buttons(buttons);

    let _ = channel_id.send_message(ctx, CreateMessage::new().embed(embed).components(vec![row])).await;
}

/// Invite : le createur mentionne un joueur → le bot lui donne acces au channel.
async fn handle_invite(ctx: &Context, component: &serenity::model::application::ComponentInteraction) {
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
        CreateInteractionResponseMessage::new().embed(embed).ephemeral(true),
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
                let now_ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs() as i64;
                if msg.timestamp.unix_timestamp() < (now_ts - 30) {
                    continue;
                }

                for mentioned in &msg.mentions {
                    if mentioned.bot || mentioned.id == author_id {
                        continue;
                    }

                    // Donner acces au channel
                    let overwrite = PermissionOverwrite {
                        allow: Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES | Permissions::READ_MESSAGE_HISTORY,
                        deny: Permissions::empty(),
                        kind: PermissionOverwriteType::Member(mentioned.id),
                    };
                    let _ = channel_id.create_permission(&ctx_clone.http, overwrite).await;

                    // Enregistrer dans l'API
                    {
                        let data = ctx_clone.data.read().await;
                        if let Some(api) = data.get::<crate::GameApiKey>() {
                            if let Ok(Some(table)) = api.get_table_by_channel(&channel_id.to_string()).await {
                                let _ = api.join_table(&table.id, &mentioned.id.to_string(), &mentioned.name).await;
                            }
                        }
                    }

                    // Notifier
                    let join_btn = CreateButton::new(JOIN_BUTTON_ID)
                        .label("Rejoindre la partie")
                        .emoji(serenity::model::channel::ReactionType::Unicode("\u{1f3b0}".into()))
                        .style(ButtonStyle::Success);

                    let embed = CreateEmbed::new()
                        .description(format!(
                            "\u{1f465} <@{}> a ete invite a la table par <@{}> !\n\nClique sur **Rejoindre** pour miser et jouer.",
                            mentioned.id, author_id
                        ))
                        .color(0x2ecc71);

                    let _ = channel_id.send_message(
                        &ctx_clone.http,
                        CreateMessage::new().embed(embed).components(vec![
                            CreateActionRow::Buttons(vec![join_btn]),
                        ]),
                    ).await;

                    info!(invited = %mentioned.name, by = %author_id, "Joueur invite a la table blackjack");
                }

                // Supprimer le message de mention
                let _ = msg.delete(&ctx_clone.http).await;
                return;
            }
        }
    });
}

/// Un joueur invite clique "Rejoindre" → affiche les boutons de mise.
async fn handle_join(ctx: &Context, component: &serenity::model::application::ComponentInteraction) {
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
        .title(format!("\u{1f0cf} {} rejoint la table !", component.user.name))
        .description("Choisis ta mise pour cette manche :")
        .color(0xF1C40F);

    let buttons = vec![
        CreateButton::new(format!("{BET_PREFIX}50")).label("50 \u{1fa99}").style(ButtonStyle::Secondary),
        CreateButton::new(format!("{BET_PREFIX}100")).label("100 \u{1fa99}").style(ButtonStyle::Primary),
        CreateButton::new(format!("{BET_PREFIX}250")).label("250 \u{1fa99}").style(ButtonStyle::Primary),
        CreateButton::new(format!("{BET_PREFIX}500")).label("500 \u{1fa99}").style(ButtonStyle::Danger),
    ];

    component
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new().embed(embed).components(vec![
                    CreateActionRow::Buttons(buttons),
                ]).ephemeral(true),
            ),
        )
        .await
        .ok();
}

/// Ferme la table (supprime le channel).
async fn handle_close_table(ctx: &Context, component: &serenity::model::application::ComponentInteraction) {
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
    drop(data);

    mgr.remove(component.user.id);

    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    if let Err(e) = component.channel_id.delete(&ctx.http).await {
        warn!(error = %e, "Echec suppression channel blackjack");
    } else {
        info!(user = %component.user.name, "Table blackjack fermee par le joueur");
    }
}
