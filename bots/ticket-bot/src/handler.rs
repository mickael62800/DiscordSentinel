use serenity::async_trait;
use serenity::model::application::Interaction;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::prelude::*;
use tracing::{error, info, warn};

use crate::api_client::ApiClient;
use crate::commands;
use crate::commands::ticket;

/// Clé pour accéder à l'ApiClient dans le TypeMap.
pub struct ApiClientKey;

impl TypeMapKey for ApiClientKey {
    type Value = ApiClient;
}

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!(bot = %ready.user.name, "Ticket bot connecte");

        // Enregistrer les slash commands globales
        if let Err(e) = serenity::model::application::Command::set_global_commands(
            &ctx.http,
            commands::all(),
        )
        .await
        {
            error!(error = %e, "Impossible d'enregistrer les slash commands");
        } else {
            info!("Slash commands enregistrees");
        }

        // Enregistrer les guilds aupres de l'API
        let data = ctx.data.read().await;
        if let Some(api) = data.get::<ApiClientKey>() {
            for guild_status in &ready.guilds {
                let guild_id = guild_status.id;
                if let Ok(guild) = guild_id.to_partial_guild(&ctx.http).await {
                    let member_count = guild.approximate_member_count.unwrap_or(0) as i32;
                    if let Err(e) = api.register_guild(
                        &guild_id.to_string(),
                        &guild.name,
                        member_count,
                    ).await {
                        warn!(error = %e, guild = %guild.name, "Erreur enregistrement guild");
                    } else {
                        info!(guild = %guild.name, "Guild enregistree");
                    }
                }
            }
        }
    }

    /// Gestion des slash commands ET des interactions composants (boutons, menus).
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        match interaction {
            Interaction::Command(command) => {
                match command.data.name.as_str() {
                    "ticket" => commands::ticket::handle(&ctx, &command).await,
                    _ => {}
                }
            }
            Interaction::Component(component) => {
                match component.data.custom_id.as_str() {
                    ticket::PANEL_BUTTON_ID => ticket::handle_panel_click(&ctx, &component).await,
                    ticket::TYPE_SELECT_ID => ticket::handle_type_select(&ctx, &component).await,
                    ticket::CLOSE_BUTTON_ID => ticket::handle_close_button(&ctx, &component).await,
                    ticket::INVITE_BUTTON_ID => ticket::handle_invite_button(&ctx, &component).await,
                    ticket::VOCAL_BUTTON_ID => ticket::handle_vocal_button(&ctx, &component).await,
                    _ => {}
                }
            }
            _ => {}
        }
    }

    /// Sync des messages dans les salons ticket vers le backend.
    /// Gere aussi les invitations par mention.
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.author.bot {
            return;
        }

        let channel_name = msg
            .channel_id
            .to_channel(&ctx.http)
            .await
            .ok()
            .and_then(|c| c.guild())
            .map(|c| c.name.clone())
            .unwrap_or_default();

        if !channel_name.starts_with("ticket-") {
            return;
        }

        // Gestion invitation par mention
        if !msg.mentions.is_empty() {
            for mentioned in &msg.mentions {
                if mentioned.bot {
                    continue;
                }
                // Ajouter la permission VIEW_CHANNEL pour l'utilisateur mentionne
                let overwrite = serenity::all::PermissionOverwrite {
                    allow: serenity::model::Permissions::VIEW_CHANNEL
                        | serenity::model::Permissions::SEND_MESSAGES
                        | serenity::model::Permissions::READ_MESSAGE_HISTORY,
                    deny: serenity::model::Permissions::empty(),
                    kind: serenity::all::PermissionOverwriteType::Member(mentioned.id),
                };
                if let Err(e) = msg.channel_id.create_permission(&ctx.http, overwrite).await {
                    warn!(error = %e, user = %mentioned.name, "Impossible d'inviter l'utilisateur");
                } else {
                    let _ = msg.channel_id.say(
                        &ctx.http,
                        format!("<@{}> a ete invite dans ce ticket.", mentioned.id),
                    ).await;
                    info!(user = %mentioned.name, channel = %channel_name, "Utilisateur invite dans le ticket");
                }
            }
        }

        // Sync message vers le backend
        let ticket_id = channel_name.trim_start_matches("ticket-");

        let data = ctx.data.read().await;
        let api = match data.get::<ApiClientKey>() {
            Some(client) => client,
            None => return,
        };

        let author_role = match msg.guild_id {
            Some(guild_id) => {
                if let Ok(member) = guild_id.member(&ctx.http, msg.author.id).await {
                    if let Ok(permissions) = member.permissions(&ctx.cache) {
                        if permissions.manage_messages() {
                            "moderator"
                        } else {
                            "user"
                        }
                    } else {
                        "user"
                    }
                } else {
                    "user"
                }
            }
            None => "user",
        };

        if let Err(e) = api
            .reply_ticket(ticket_id, &msg.content, &msg.author.name, author_role)
            .await
        {
            error!(error = %e, ticket_id = %ticket_id, "Erreur sync message vers backend");
        }
    }
}
