use serenity::all::{
    CommandInteraction, CommandOptionType, Context, CreateCommand, CreateCommandOption,
    CreateInteractionResponse, CreateInteractionResponseMessage,
};
use tracing::{error, warn};

use crate::shared::embeds::{action_emoji, info_embed};

use super::ModerationApiKey;
use crate::shared::discord_helpers::edit_response_text;

pub fn register() -> CreateCommand {
    CreateCommand::new("history")
        .description("Voir l'historique des sanctions d'un utilisateur")
        .default_member_permissions(serenity::all::Permissions::MODERATE_MEMBERS)
        .add_option(CreateCommandOption::new(
            CommandOptionType::User,
            "user",
            "Utilisateur a verifier (ou utilise user_id)",
        ))
        .add_option(CreateCommandOption::new(
            CommandOptionType::String,
            "user_id",
            "ID de l'utilisateur (ex. membre parti / banni)",
        ))
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    let target_id = match super::resolve_target_user_id(command, "user") {
        Some(id) => id,
        None => {
            edit_response_text(
                ctx,
                command,
                "Indique un membre (`user`) ou un identifiant (`user_id`).",
            )
            .await;
            return;
        }
    };

    let guild_id = match command.guild_id {
        Some(id) => id,
        None => {
            edit_response_text(ctx, command, "Commande serveur uniquement.").await;
            return;
        }
    };

    let target = target_id.to_user(&ctx.http).await.ok();
    let username = target
        .as_ref()
        .map(|u| u.name.as_str())
        .unwrap_or("inconnu");

    let data = ctx.data.read().await;
    let api = match data.get::<ModerationApiKey>() {
        Some(a) => a,
        None => {
            tracing::error!("ModerationApiKey manquant");
            return;
        }
    };

    match api
        .get_history(&guild_id.to_string(), &target_id.to_string())
        .await
    {
        Ok(history) => {
            let description = if history.actions.is_empty() {
                "Aucune sanction enregistree.".to_string()
            } else {
                history
                    .actions
                    .iter()
                    .take(10)
                    .enumerate()
                    .map(|(i, action)| {
                        format!(
                            "{}. {} **{}** — {}",
                            i + 1,
                            action_emoji(&action.action_type),
                            action.action_type,
                            action.reason
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            };

            let embed = info_embed(format!("📋 Historique de @{username}"))
                .description(description)
                .field("Warns", history.total_warns.to_string(), true)
                .field("Mutes", history.total_mutes.to_string(), true)
                .field("Bans", history.total_bans.to_string(), true);

            if let Err(e) = command
                .create_response(
                    &ctx.http,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .embed(embed)
                            .ephemeral(true),
                    ),
                )
                .await
            {
                warn!(error = %e, "Failed to send history response");
            }
        }
        Err(e) => {
            error!(error = %e, "Erreur recuperation historique");
            edit_response_text(ctx, command, &format!("Erreur : {e}")).await;
        }
    }
}
