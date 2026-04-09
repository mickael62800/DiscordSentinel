use std::sync::Arc;

use serenity::async_trait;
use serenity::builder::{
    CreateActionRow, CreateButton, CreateEmbed, CreateEmbedFooter, CreateInteractionResponse,
    CreateInteractionResponseMessage, CreateMessage, EditChannel,
};
use serenity::model::application::{ButtonStyle, Interaction};
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::model::guild::Member;
use serenity::model::id::{ChannelId, GuildId, RoleId};
use serenity::model::user::User;
use serenity::prelude::*;
use tracing::{info, warn};

use sentinel_shared::heartbeat::{ApiClientKey, register_guilds};

use crate::api_client::WelcomeApiClient;
use crate::template;

pub const RULES_ACCEPT_ID: &str = "sentinel_rules_accept";

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!(bot = %ready.user.name, guilds = ready.guilds.len(), "Welcome bot connecte");
        register_guilds(&ctx, &ready).await;
    }

    async fn guild_member_addition(&self, ctx: Context, new_member: Member) {
        let guild_id = new_member.guild_id;
        let user_id = new_member.user.id;

        let data = ctx.data.read().await;
        let base = match data.get::<ApiClientKey>() {
            Some(b) => Arc::clone(b),
            None => return,
        };
        drop(data);

        let api = WelcomeApiClient::new(base.clone());
        let config = match api.get_config(&guild_id.to_string()).await {
            Ok(c) => c,
            Err(e) => {
                warn!(error = %e, "Echec chargement config welcome");
                return;
            }
        };

        let guild_name = guild_id
            .to_partial_guild(&ctx.http)
            .await
            .map(|g| g.name.clone())
            .unwrap_or_else(|_| "Serveur".into());

        let member_count = guild_id
            .to_partial_guild_with_counts(&ctx.http)
            .await
            .map(|g| g.approximate_member_count.unwrap_or(0))
            .unwrap_or(0);

        // ── Detecter si c'est un retour (membre deja connu) ──
        let is_rejoin = api.is_known_member(&guild_id.to_string(), &user_id.to_string()).await;

        // ── Message de bienvenue ──
        if config.welcome_enabled {
            if let Some(ch_id) = &config.welcome_channel_id {
                if let Ok(ch) = ch_id.parse::<u64>() {
                    let channel = ChannelId::new(ch);

                    // Choisir le message : retour ou premiere fois
                    let msg_template = if is_rejoin {
                        &config.rejoin_message
                    } else {
                        &config.welcome_message
                    };

                    let text = template::render(
                        msg_template,
                        &user_id.to_string(),
                        &new_member.user.name,
                        &guild_name,
                        member_count,
                        None,
                    );

                    let title = if is_rejoin { "Bon retour !" } else { "Bienvenue !" };
                    let color = template::parse_color(&config.welcome_embed_color);
                    let embed = CreateEmbed::new()
                        .title(title)
                        .description(&text)
                        .color(color)
                        .thumbnail(new_member.user.face())
                        .footer(CreateEmbedFooter::new(format!("{} membres", member_count)));

                    if let Err(e) = channel.send_message(&ctx.http, CreateMessage::new().embed(embed)).await {
                        warn!(error = %e, "Echec envoi message bienvenue");
                    } else {
                        info!(
                            user = %new_member.user.name,
                            guild = %guild_name,
                            rejoin = is_rejoin,
                            "Message de {} envoye",
                            if is_rejoin { "retour" } else { "bienvenue" }
                        );
                    }
                }
            }
        }

        // ── DM de bienvenue ──
        if config.welcome_dm_enabled {
            let dm_text = template::render(
                &config.welcome_dm_message,
                &user_id.to_string(),
                &new_member.user.name,
                &guild_name,
                member_count,
                None,
            );

            if let Ok(dm_channel) = new_member.user.create_dm_channel(&ctx.http).await {
                if let Err(e) = dm_channel.send_message(&ctx.http, CreateMessage::new().content(&dm_text)).await {
                    warn!(error = %e, user = %new_member.user.name, "Echec envoi DM bienvenue");
                }
            }
        }

        // ── Compteur de membres ──
        if config.counter_enabled {
            if let Some(ch_id) = &config.counter_channel_id {
                if let Ok(ch) = ch_id.parse::<u64>() {
                    let name = config.counter_format.replace("{count}", &member_count.to_string());
                    let edit = EditChannel::new().name(&name);
                    if let Err(e) = ChannelId::new(ch).edit(&ctx.http, edit).await {
                        warn!(error = %e, "Echec mise a jour compteur membres");
                    }
                }
            }
        }

        // ── Log ──
        base.send_log("info", &guild_id.to_string(), &format!(
            "Nouveau membre : {} ({})", new_member.user.name, user_id
        ));
    }

    async fn guild_member_removal(
        &self,
        ctx: Context,
        guild_id: GuildId,
        user: User,
        _member: Option<Member>,
    ) {
        let data = ctx.data.read().await;
        let base = match data.get::<ApiClientKey>() {
            Some(b) => Arc::clone(b),
            None => return,
        };
        drop(data);

        let api = WelcomeApiClient::new(base.clone());
        let config = match api.get_config(&guild_id.to_string()).await {
            Ok(c) => c,
            Err(_) => return,
        };

        if !config.leave_enabled {
            return;
        }

        let ch_id = match &config.leave_channel_id {
            Some(c) => c,
            None => return,
        };

        let ch = match ch_id.parse::<u64>() {
            Ok(c) => ChannelId::new(c),
            Err(_) => return,
        };

        let guild_name = guild_id
            .to_partial_guild(&ctx.http)
            .await
            .map(|g| g.name.clone())
            .unwrap_or_else(|_| "Serveur".into());

        let member_count = guild_id
            .to_partial_guild_with_counts(&ctx.http)
            .await
            .map(|g| g.approximate_member_count.unwrap_or(0))
            .unwrap_or(0);

        let text = template::render(
            &config.leave_message,
            &user.id.to_string(),
            &user.name,
            &guild_name,
            member_count,
            None,
        );

        let embed = CreateEmbed::new()
            .description(&text)
            .color(0xe74c3c);

        if let Err(e) = ch.send_message(&ctx.http, CreateMessage::new().embed(embed)).await {
            warn!(error = %e, "Echec envoi message depart");
        }

        // Mise a jour compteur
        if config.counter_enabled {
            if let Some(counter_ch) = &config.counter_channel_id {
                if let Ok(c) = counter_ch.parse::<u64>() {
                    let name = config.counter_format.replace("{count}", &member_count.to_string());
                    let _ = ChannelId::new(c).edit(&ctx.http, EditChannel::new().name(&name)).await;
                }
            }
        }
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::Component(component) = interaction {
            if component.data.custom_id == RULES_ACCEPT_ID {
                handle_rules_accept(&ctx, &component).await;
            }
        }
    }
}

/// Gere le clic sur le bouton "J'accepte les regles".
async fn handle_rules_accept(
    ctx: &Context,
    component: &serenity::model::application::ComponentInteraction,
) {
    let guild_id = match component.guild_id {
        Some(g) => g,
        None => return,
    };

    let data = ctx.data.read().await;
    let base = match data.get::<ApiClientKey>() {
        Some(b) => Arc::clone(b),
        None => return,
    };
    drop(data);

    let api = WelcomeApiClient::new(base);
    let config = match api.get_config(&guild_id.to_string()).await {
        Ok(c) => c,
        Err(_) => return,
    };

    if !config.rules_enabled {
        return;
    }

    let role_id = match &config.rules_role_id {
        Some(r) => match r.parse::<u64>() {
            Ok(id) => RoleId::new(id),
            Err(_) => return,
        },
        None => return,
    };

    // Assigner le role
    if let Err(e) = ctx.http.add_member_role(
        guild_id,
        component.user.id,
        role_id,
        Some("Reglement accepte"),
    ).await {
        warn!(error = %e, "Echec assignation role reglement");
        let resp = CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new()
                .content("Erreur lors de l'assignation du role.")
                .ephemeral(true),
        );
        let _ = component.create_response(&ctx.http, resp).await;
        return;
    }

    let resp = CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new()
            .content("Reglement accepte ! Bienvenue sur le serveur.")
            .ephemeral(true),
    );
    if let Err(e) = component.create_response(&ctx.http, resp).await {
        warn!(error = %e, "Echec reponse acceptation reglement");
    }

    info!(user = %component.user.name, guild = %guild_id, "Reglement accepte");
}

/// Cree le message de reglement avec le bouton d'acceptation.
/// Appele par la commande /welcome rules-post.
pub async fn post_rules_message(
    ctx: &Context,
    channel_id: ChannelId,
    message: &str,
    button_label: &str,
) -> Result<Message, serenity::Error> {
    let embed = CreateEmbed::new()
        .title("Reglement du serveur")
        .description(message)
        .color(0x2ecc71);

    let button = CreateButton::new(RULES_ACCEPT_ID)
        .label(button_label)
        .style(ButtonStyle::Success);

    let row = CreateActionRow::Buttons(vec![button]);

    channel_id
        .send_message(
            &ctx.http,
            CreateMessage::new().embed(embed).components(vec![row]),
        )
        .await
}
