//! EventHandler unifie — dispatche vers les modules.

use serenity::async_trait;
use serenity::model::application::Interaction;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::prelude::*;
use tracing::info;

use sentinel_shared::heartbeat::register_guilds;

use crate::modules;

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!(
            bot = %ready.user.name,
            guilds = ready.guilds.len(),
            "Sentinel Bot connecte"
        );

        register_guilds(&ctx, &ready).await;

        // Enregistrer toutes les commandes slash en une fois.
        let mut commands = Vec::new();
        commands.extend(modules::cleanup::register_commands());
        commands.extend(modules::games::register_commands());

        if let Err(e) = serenity::model::application::Command::set_global_commands(
            &ctx.http,
            commands,
        )
        .await
        {
            tracing::error!(error = %e, "Erreur enregistrement commandes");
        } else {
            info!("Slash commands enregistrees (sentinel-bot unifie)");
        }
    }

    async fn message(&self, ctx: Context, msg: Message) {
        if msg.author.bot {
            return;
        }
        // Games : detection mentions #Jeu
        modules::games::on_message(&ctx, &msg).await;
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        match interaction {
            Interaction::Command(command) => {
                let name = command.data.name.as_str();
                match name {
                    "purge" | "cleanup" => modules::cleanup::handle_command(&ctx, &command).await,
                    "game" => modules::games::handle_command(&ctx, &command).await,
                    _ => {}
                }
            }
            _ => {}
        }
    }
}
