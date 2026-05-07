pub mod constants;
pub mod helpers;
pub mod panel;
pub mod close;
pub mod interactions;

pub use constants::*;
pub use helpers::*;
pub use panel::*;
pub use close::*;
pub use interactions::*;

use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context,
    CreateCommand, CreateCommandOption,
};
use tracing::error;

/// Enregistre les commandes /ticket (public) et /ticket-admin (staff).
pub fn register() -> Vec<CreateCommand> {
    vec![register_public(), register_admin()]
}

fn register_public() -> CreateCommand {
    CreateCommand::new("ticket")
        .description("Gestion des tickets de support")
        .default_member_permissions(serenity::all::Permissions::empty())
        .add_option(CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "close",
            "Fermer le ticket du salon actuel",
        ))
}

fn register_admin() -> CreateCommand {
    CreateCommand::new("ticket-admin")
        .description("Administration des tickets (staff)")
        .default_member_permissions(serenity::all::Permissions::MANAGE_CHANNELS)
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "panel",
                "Deployer le panneau de creation de ticket dans ce salon",
            ),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "invite",
                "Inviter un membre dans ce ticket",
            )
            .add_sub_option(
                CreateCommandOption::new(
                    CommandOptionType::User,
                    "membre",
                    "Membre a inviter",
                )
                .required(true),
            ),
        )
}

/// Dispatch la slash command vers la bonne sous-commande.
pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    let sub = &command.data.options[0];
    let top = command.data.name.as_str();
    let result = match (top, sub.name.as_str()) {
        ("ticket", "close") => handle_close(ctx, command).await,
        ("ticket-admin", "panel") => handle_panel(ctx, command).await,
        ("ticket-admin", "invite") => handle_invite(ctx, command).await,
        _ => reply(ctx, command, "Sous-commande inconnue.").await,
    };

    if let Err(e) = result {
        error!(error = %e, "Erreur commande ticket");
    }
}

/// /ticket panel — Envoie le message permanent avec le bouton "Creer un ticket"
async fn handle_panel(
    ctx: &Context,
    command: &CommandInteraction,
) -> Result<(), serenity::Error> {
    if let Some(guild_id) = command.guild_id {
        let is_staff = helpers::is_staff_member(ctx, guild_id, command.user.id).await;
        if !is_staff {
            return reply(ctx, command, "Seuls les administrateurs et moderateurs peuvent deployer le panel.").await;
        }
    } else {
        return reply(ctx, command, "Cette commande doit etre utilisee dans un serveur.").await;
    }

    command.channel_id.send_message(&ctx.http, build_panel_message()).await?;
    reply(ctx, command, "Panneau de tickets deploye !").await
}

/// Commande /ticket close
async fn handle_close(
    ctx: &Context,
    command: &CommandInteraction,
) -> Result<(), serenity::Error> {
    use serenity::model::channel::ChannelType;
    use crate::shared::heartbeat::ApiClientKey;
    use super::api_client::ApiClient;
    use tracing::{info, warn};

    let channel_name = command
        .channel_id
        .to_channel(&ctx.http)
        .await
        .ok()
        .and_then(|c| c.guild())
        .map(|c| c.name.clone())
        .unwrap_or_default();

    if !channel_name.starts_with("ticket-") {
        return reply(ctx, command, "Cette commande ne fonctionne que dans un salon de ticket.").await;
    }

    if let Some(guild_id) = command.guild_id {
        let is_staff = helpers::is_staff_member(ctx, guild_id, command.user.id).await;
        if !is_staff {
            let topic = command.channel_id
                .to_channel(&ctx.http).await.ok()
                .and_then(|c| c.guild())
                .and_then(|c| c.topic)
                .unwrap_or_default();
            let is_author = topic.contains(&command.user.id.to_string());
            if !is_author {
                return reply(ctx, command, "Seul le staff ou l'auteur du ticket peut le fermer.").await;
            }
        }
    }

    let ticket_id = get_ticket_id_from_channel(ctx, command.channel_id).await;

    reply(ctx, command, "Fermeture du ticket...").await?;

    let data = ctx.data.read().await;
    if let Some(base) = data.get::<ApiClientKey>() {
        if let Some(ref id) = ticket_id {
            if let Some(grpc) = data.get::<crate::shared::grpc_client::GrpcClientKey>() {
                let api = ApiClient::new(base.clone(), grpc.clone());
                if let Err(e) = api.close_ticket(id).await {
                    error!(error = %e, ticket_id = %id, "Erreur fermeture ticket API");
                }
            }
        } else {
            warn!(channel = %channel_name, "Impossible de trouver l'UUID du ticket dans le topic du salon");
        }

        base.send_log(
            "info",
            &command.guild_id.map(|g| g.to_string()).unwrap_or_default(),
            &format!(
                "Ticket ferme : {} (id: {}) par {}",
                channel_name,
                ticket_id.as_deref().unwrap_or("inconnu"),
                command.user.name
            ),
        );
    }

    if let Some(guild_id) = command.guild_id {
        let vocal_name = format!("vocal-{}", channel_name);
        if let Ok(channels) = guild_id.channels(&ctx.http).await {
            for (ch_id, ch) in &channels {
                if ch.kind == ChannelType::Voice && ch.name == vocal_name {
                    if let Err(e) = ch_id.delete(&ctx.http).await {
                        warn!(error = %e, vocal = %vocal_name, "Impossible de supprimer le salon vocal du ticket");
                    } else {
                        info!(vocal = %vocal_name, "Salon vocal du ticket supprime");
                    }
                }
            }
        }
    }

    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
    if let Err(e) = command.channel_id.delete(&ctx.http).await {
        warn!(error = %e, "Failed to delete ticket channel");
    }

    Ok(())
}

/// Commande /ticket invite <membre>
async fn handle_invite(
    ctx: &Context,
    command: &CommandInteraction,
) -> Result<(), serenity::Error> {
    use serenity::all::PermissionOverwrite;
    use serenity::all::PermissionOverwriteType;
    use serenity::model::Permissions;

    let channel_name = command
        .channel_id
        .to_channel(&ctx.http)
        .await
        .ok()
        .and_then(|c| c.guild())
        .map(|c| c.name.clone())
        .unwrap_or_default();

    if !channel_name.starts_with("ticket-") {
        return reply(ctx, command, "Cette commande ne fonctionne que dans un salon de ticket.").await;
    }

    if let Some(guild_id) = command.guild_id {
        let is_staff = helpers::is_staff_member(ctx, guild_id, command.user.id).await;
        if !is_staff {
            return reply(ctx, command, "Seul le staff peut inviter des membres dans un ticket.").await;
        }
    }

    let options = get_sub_options(command);
    let user_id = options
        .iter()
        .find(|o| o.name == "membre")
        .and_then(|o| match &o.value {
            CommandDataOptionValue::User(id) => Some(*id),
            _ => None,
        });

    let user_id = match user_id {
        Some(id) => id,
        None => return reply(ctx, command, "Veuillez specifier un membre.").await,
    };

    let overwrite = PermissionOverwrite {
        allow: Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES | Permissions::READ_MESSAGE_HISTORY,
        deny: Permissions::empty(),
        kind: PermissionOverwriteType::Member(user_id),
    };

    command.channel_id.create_permission(&ctx.http, overwrite).await?;

    reply(
        ctx,
        command,
        &format!("<@{user_id}> a ete invite dans ce ticket."),
    )
    .await
}
