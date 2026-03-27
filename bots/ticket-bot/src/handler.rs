use serenity::async_trait;
use serenity::model::application::Interaction;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::prelude::*;
use tracing::{error, info, warn};

use crate::api_client::ApiClient;
use crate::commands;

/// Clé pour accéder à l'ApiClient dans le TypeMap.
pub struct ApiClientKey;

impl TypeMapKey for ApiClientKey {
    type Value = ApiClient;
}

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!(bot = %ready.user.name, "Ticket bot connecté");

        // Enregistrer les slash commands globales
        if let Err(e) = serenity::model::application::Command::set_global_commands(
            &ctx.http,
            commands::all(),
        )
        .await
        {
            error!(error = %e, "Impossible d'enregistrer les slash commands");
        } else {
            info!("Slash commands enregistrées");
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

    /// Gestion des slash commands.
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::Command(command) = interaction {
            match command.data.name.as_str() {
                "ticket" => commands::ticket::handle(&ctx, &command).await,
                _ => {}
            }
        }
    }

    /// Sync des messages dans les threads ticket vers le backend.
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.author.bot {
            return;
        }

        // Vérifier si le message est dans un thread de ticket
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

        let ticket_id = channel_name.trim_start_matches("ticket-");

        // Envoyer le message au backend
        let data = ctx.data.read().await;
        let api = match data.get::<ApiClientKey>() {
            Some(client) => client,
            None => return,
        };

        // Déterminer le rôle de l'auteur (modérateur si permission MANAGE_MESSAGES, sinon user)
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
