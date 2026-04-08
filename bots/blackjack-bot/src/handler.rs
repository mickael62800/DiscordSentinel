use serenity::async_trait;
use serenity::model::application::Interaction;
use serenity::model::gateway::Ready;
use serenity::prelude::*;
use tracing::{error, info};

use sentinel_shared::heartbeat::register_guilds;

use crate::commands;

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
            info!("Slash commands enregistrees : blackjack");
        }
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        match interaction {
            Interaction::Command(command) => {
                if let Some(guild_id) = command.guild_id {
                    let data = ctx.data.read().await;
                    if let Some(api) = data.get::<sentinel_shared::heartbeat::ApiClientKey>() {
                        if !sentinel_shared::discord_helpers::is_bot_enabled(api, &guild_id.to_string()).await {
                            return;
                        }
                    }
                }

                match command.data.name.as_str() {
                    "blackjack" => commands::blackjack::handle(&ctx, &command).await,
                    _ => {}
                }
            }
            Interaction::Component(component) => {
                let custom_id = &component.data.custom_id;
                if custom_id.starts_with("bj_hit:")
                    || custom_id.starts_with("bj_stand:")
                    || custom_id.starts_with("bj_double:")
                {
                    commands::blackjack::handle_component(&ctx, &component).await;
                }
            }
            _ => {}
        }
    }
}
