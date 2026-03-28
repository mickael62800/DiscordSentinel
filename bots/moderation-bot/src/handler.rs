use serenity::async_trait;
use serenity::model::application::Interaction;
use serenity::model::gateway::Ready;
use serenity::prelude::*;
use tracing::{error, info, warn};

use crate::api_client::ApiClient;
use crate::commands;

pub struct ApiClientKey;
impl TypeMapKey for ApiClientKey {
    type Value = ApiClient;
}

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!(bot = %ready.user.name, "Moderation bot connecté");
        {
            let data = ctx.data.read().await;
            if let Some(api) = data.get::<ApiClientKey>() {
                api.send_log("info", "", "Moderation bot demarre");
            }
        }

        if let Err(e) = serenity::model::application::Command::set_global_commands(
            &ctx.http,
            commands::all(),
        )
        .await
        {
            error!(error = %e, "Impossible d'enregistrer les slash commands");
        } else {
            info!("Slash commands enregistrées : warn, mute, unmute, ban, unban, history");
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

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::Command(command) = interaction {
            let cmd_name = command.data.name.clone();
            let moderator = command.user.name.clone();
            let guild_id = command.guild_id.map(|g| g.to_string()).unwrap_or_default();

            match cmd_name.as_str() {
                "warn" => commands::warn::handle(&ctx, &command).await,
                "mute" => commands::mute::handle(&ctx, &command).await,
                "unmute" => commands::mute::handle_unmute(&ctx, &command).await,
                "ban" => commands::ban::handle(&ctx, &command).await,
                "unban" => commands::ban::handle_unban(&ctx, &command).await,
                "history" => commands::history::handle(&ctx, &command).await,
                _ => {}
            }

            let data = ctx.data.read().await;
            if let Some(api) = data.get::<ApiClientKey>() {
                api.send_log("info", &guild_id, &format!("Commande /{} executee par {}", cmd_name, moderator));
            }
        }
    }
}
