//! EventHandler unifie — dispatche vers les modules.

use serenity::async_trait;
use serenity::model::application::Interaction;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::model::guild::Member;
use serenity::model::id::GuildId;
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
        commands.extend(modules::roles::register_commands());
        commands.extend(modules::audit::register_commands());

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
        modules::games::on_message(&ctx, &msg).await;
    }

    async fn guild_member_addition(&self, ctx: Context, new_member: Member) {
        modules::welcome::on_member_add(&ctx, &new_member).await;
    }

    async fn guild_member_removal(
        &self,
        ctx: Context,
        guild_id: GuildId,
        user: serenity::model::user::User,
        _member: Option<Member>,
    ) {
        modules::welcome::on_member_remove(&ctx, guild_id, &user).await;
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        match interaction {
            Interaction::Command(command) => {
                let name = command.data.name.as_str();
                match name {
                    "purge" | "cleanup" => modules::cleanup::handle_command(&ctx, &command).await,
                    "game" => modules::games::handle_command(&ctx, &command).await,
                    "roles-panel" => modules::roles::handle_command(&ctx, &command).await,
                    "audit" => modules::audit::handle_command(&ctx, &command).await,
                    _ => {}
                }
            }
            Interaction::Component(component) => {
                let cid = component.data.custom_id.as_str();
                if modules::welcome::handles_component(cid) {
                    modules::welcome::on_component(&ctx, &component).await;
                }
            }
            _ => {}
        }
    }
}
