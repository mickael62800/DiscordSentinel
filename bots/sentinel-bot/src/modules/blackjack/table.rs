//! Cycle de vie d'une table blackjack : creation, invitation, join, fermeture.

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

use super::{ChannelManagerKey, GameApiKey, BET_PREFIX, CLOSE_TABLE_ID, INVITE_BUTTON_ID, JOIN_BUTTON_ID};

/// Handler Redis : `blackjack_table_closed` depuis web -> edit l'embed
/// Discord pour signaler la fermeture (gris + retire boutons).
pub async fn handle_redis_event(ctx: &Context, payload: &str) {
    use serenity::all::{ChannelId, GetMessages, MessageId};

    let event: serde_json::Value = match serde_json::from_str(payload) {
        Ok(v) => v,
        Err(_) => return,
    };
    if event.get("event").and_then(|v| v.as_str()) != Some("blackjack_table_closed") {
        return;
    }
    let data = match event.get("data") {
        Some(d) => d,
        None => return,
    };
    if data.get("actor").and_then(|a| a.get("source")).and_then(|s| s.as_str())
        != Some("web")
    {
        return;
    }
    let action_id = match data.get("action_id").and_then(|v| v.as_str()) {
        Some(a) if !a.is_empty() => a,
        _ => return,
    };

    let api = {
        let lock = ctx.data.read().await;
        match lock.get::<sentinel_shared::heartbeat::ApiClientKey>() {
            Some(a) => a.clone(),
            None => return,
        }
    };

    #[derive(serde::Deserialize)]
    struct Mapping {
        kind: String,
        channel_id: String,
        message_id: String,
    }
    let mappings: Vec<Mapping> = match api
        .get_json(&format!("/api/discord-messages/{action_id}"))
        .await
    {
        Ok(list) => list,
        Err(e) => {
            warn!(error = %e, action_id, "Echec fetch mapping blackjack_table");
            return;
        }
    };
    let m = match mappings.into_iter().find(|m| m.kind == "blackjack_table") {
        Some(m) => m,
        None => return,
    };

    let channel_id = match m.channel_id.parse::<u64>() {
        Ok(v) => ChannelId::new(v),
        Err(_) => return,
    };
    let msg_id = match m.message_id.parse::<u64>() {
        Ok(v) => MessageId::new(v),
        Err(_) => return,
    };

    if let Ok(messages) = channel_id
        .messages(&ctx.http, GetMessages::new().limit(1).around(msg_id))
        .await
    {
        if let Some(original) = messages.into_iter().find(|m| m.id == msg_id) {
            if let Some(existing) = original.embeds.first() {
                let new_embed = CreateEmbed::from(existing.clone())
                    .color(0x95A5A6)
                    .footer(CreateEmbedFooter::new(
                        "\u{1f512} Table fermee depuis la web admin",
                    ));
                if let Err(e) = channel_id
                    .edit_message(
                        &ctx.http,
                        msg_id,
                        serenity::builder::EditMessage::new()
                            .embed(new_embed)
                            .components(vec![]),
                    )
                    .await
                {
                    warn!(error = %e, %channel_id, %msg_id, "Echec edit embed blackjack table apres close web");
                }
            }
        }
    }
    info!(action_id, "Embed blackjack table grise (close via web)");
}

/// Clic sur "Jouer au Blackjack" du panel -> cree un channel prive.
pub(super) async fn handle_panel_click(ctx: &Context, component: &ComponentInteraction) {
    let guild_id = match component.guild_id {
        Some(g) => g,
        None => return,
    };
    let user_id = component.user.id;

    // Verifier si deja une table ouverte — on verifie aussi que le channel
    // existe reellement pour detecter les entrees orphelines dans le
    // ChannelManager in-memory (ex: channel supprime manuellement, cache
    // non nettoye apres crash du bot, etc.).
    {
        let data = ctx.data.read().await;
        if let Some(mgr) = data.get::<ChannelManagerKey>() {
            if let Some(table) = mgr.get(user_id) {
                // Le channel existe-t-il toujours ?
                let channel_still_exists = table
                    .channel_id
                    .to_channel(&ctx.http)
                    .await
                    .is_ok();
                if channel_still_exists {
                    let resp = CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .content(format!(
                                "Tu as deja une table ouverte ! <#{}>",
                                table.channel_id
                            ))
                            .ephemeral(true),
                    );
                    let _ = component.create_response(&ctx.http, resp).await;
                    return;
                }
                // Sinon : entry orpheline -> on la purge et on continue
                // vers la creation d'une nouvelle table.
                warn!(
                    user_id = %user_id,
                    channel_id = %table.channel_id,
                    "Channel fantome detecte dans ChannelManager, purge"
                );
                mgr.remove(user_id);
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

    // Creer la table en DB via l'API. Capture l'id (UUID) pour pouvoir
    // ensuite enregistrer le mapping `discord_action_messages` (sync
    // bilateral : close depuis web -> edit l'embed Discord).
    let table_uuid: Option<uuid::Uuid> = {
        let data = ctx.data.read().await;
        if let Some(api) = data.get::<GameApiKey>() {
            match api
                .create_table(
                    &guild_id.to_string(),
                    &channel.id.to_string(),
                    &user_id.to_string(),
                    &component.user.name,
                )
                .await
            {
                Ok(t) => uuid::Uuid::parse_str(&t.id).ok(),
                Err(e) => {
                    warn!(error = %e, "Echec creation table API (continue quand meme)");
                    None
                }
            }
        } else {
            None
        }
    };

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

    // Paliers de mise affiches dans l'UI. L'API valide min_bet/max_bet
    // en config guild — si un palier est hors range, StartGame retourne
    // une erreur claire au joueur. Idealement ces paliers viendraient de
    // la config guild (futur: RPC GetBlackjackConfig).
    const BET_TIERS: &[(u64, ButtonStyle)] = &[
        (50, ButtonStyle::Secondary),
        (100, ButtonStyle::Primary),
        (250, ButtonStyle::Primary),
        (500, ButtonStyle::Danger),
    ];
    let bet_buttons: Vec<CreateButton> = BET_TIERS
        .iter()
        .map(|(amount, style)| {
            CreateButton::new(format!("{BET_PREFIX}{amount}"))
                .label(format!("{amount} \u{1fa99}"))
                .style(*style)
        })
        .collect();
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

    let posted_msg_id = match channel
        .id
        .send_message(
            &ctx.http,
            CreateMessage::new().embed(embed).components(vec![row1, row2]),
        )
        .await
    {
        Ok(m) => Some(m.id),
        Err(e) => {
            warn!(error = %e, "Echec envoi embed table blackjack");
            None
        }
    };

    // Sync bilateral : enregistre le mapping pour permettre l'edit Discord
    // si l'admin ferme la table depuis la web admin.
    if let (Some(uuid), Some(msg_id)) = (table_uuid, posted_msg_id) {
        let data = ctx.data.read().await;
        if let Some(api) = data.get::<sentinel_shared::heartbeat::ApiClientKey>() {
            let api = std::sync::Arc::clone(api);
            let g = guild_id.to_string();
            let c = channel.id.to_string();
            let m = msg_id.to_string();
            drop(data);
            crate::sync::register_action_message(
                &api,
                uuid,
                crate::sync::kinds::BLACKJACK_TABLE,
                &g,
                &c,
                &m,
            )
            .await;
        }
    }

    info!(
        user = %component.user.name,
        channel = %channel.id,
        "Table blackjack multijoueur ouverte"
    );
}

/// Invite : le createur mentionne un joueur -> le bot lui donne acces au channel.
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
                        if let Some(api) = data.get::<GameApiKey>() {
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

/// Un joueur invite clique "Rejoindre" -> affiche les boutons de mise.
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
    let api = data.get::<GameApiKey>().cloned();
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
