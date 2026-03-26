use serenity::async_trait;
use serenity::model::application::Interaction;
use serenity::model::gateway::Ready;
use serenity::prelude::*;
use tracing::{error, info};

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
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::Command(command) = interaction {
            match command.data.name.as_str() {
                "warn" => commands::warn::handle(&ctx, &command).await,
                "mute" => commands::mute::handle(&ctx, &command).await,
                "unmute" => commands::mute::handle_unmute(&ctx, &command).await,
                "ban" => commands::ban::handle(&ctx, &command).await,
                "unban" => commands::ban::handle_unban(&ctx, &command).await,
                "history" => commands::history::handle(&ctx, &command).await,
                _ => {}
            }
        }
    }
}
