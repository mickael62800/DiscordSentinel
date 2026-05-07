use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context, CreateCommand,
    CreateCommandOption, CreateInteractionResponse, CreateInteractionResponseMessage,
};
use tracing::{error, warn};

use crate::shared::embeds::{info_embed, success_embed};

use super::ModerationApiKey;
use crate::shared::discord_helpers::edit_response_text;

pub fn register() -> CreateCommand {
    CreateCommand::new("review")
        .description("File de relecture des actions de moderation (seconde opinion)")
        .default_member_permissions(serenity::all::Permissions::MODERATE_MEMBERS)
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "add",
                "Ajouter une action a la file de relecture",
            )
            .add_sub_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "action_id",
                    "ID de l'action (UUID)",
                )
                .required(true),
            )
            .add_sub_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "reason",
                    "Raison de la demande (optionnel)",
                ),
            ),
        )
        .add_option(CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "list",
            "Lister les reviews en attente",
        ))
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "resolve",
                "Resoudre une review (senior mod uniquement)",
            )
            .add_sub_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "review_id",
                    "ID de la review (UUID)",
                )
                .required(true),
            )
            .add_sub_option(
                CreateCommandOption::new(CommandOptionType::String, "status", "Statut")
                    .required(true)
                    .add_string_choice("Approved", "approved")
                    .add_string_choice("Rejected", "rejected")
                    .add_string_choice("Changed", "changed"),
            )
            .add_sub_option(CreateCommandOption::new(
                CommandOptionType::String,
                "notes",
                "Notes du relecteur",
            )),
        )
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    let sub = command
        .data
        .options
        .iter()
        .find_map(|o| match &o.value {
            CommandDataOptionValue::SubCommand(inner) => Some((o.name.as_str(), inner.as_slice())),
            _ => None,
        });

    let (sub_name, sub_opts) = match sub {
        Some(s) => s,
        None => {
            edit_response_text(ctx, command, "Sous-commande manquante.").await;
            return;
        }
    };

    match sub_name {
        "add" => handle_add(ctx, command, sub_opts).await,
        "list" => handle_list(ctx, command).await,
        "resolve" => handle_resolve(ctx, command, sub_opts).await,
        _ => edit_response_text(ctx, command, "Sous-commande inconnue.").await,
    }
}

async fn handle_add(
    ctx: &Context,
    command: &CommandInteraction,
    opts: &[serenity::model::application::CommandDataOption],
) {
    let action_id = opts
        .iter()
        .find(|o| o.name == "action_id")
        .and_then(|o| match &o.value {
            CommandDataOptionValue::String(s) => Some(s.as_str()),
            _ => None,
        })
        .unwrap_or("");
    let reason = opts
        .iter()
        .find(|o| o.name == "reason")
        .and_then(|o| match &o.value {
            CommandDataOptionValue::String(s) => Some(s.as_str()),
            _ => None,
        });

    let guild_id = match command.guild_id {
        Some(id) => id,
        None => {
            edit_response_text(ctx, command, "Commande serveur uniquement.").await;
            return;
        }
    };

    if action_id.is_empty() {
        edit_response_text(ctx, command, "action_id requis.").await;
        return;
    }

    let data = ctx.data.read().await;
    let api = match data.get::<ModerationApiKey>() {
        Some(a) => a,
        None => return,
    };

    match api
        .add_review(
            action_id,
            &guild_id.to_string(),
            &command.user.id.to_string(),
            &command.user.name,
            reason,
        )
        .await
    {
        Ok(entry) => {
            let embed = success_embed("\u{1f4cb} Review ajoutee a la queue")
                .field("ID", format!("`{}`", short_id(&entry.id)), true)
                .field("Action", format!("`{}`", short_id(&entry.action_id)), true)
                .field(
                    "Raison",
                    reason.unwrap_or("_non specifiee_").to_string(),
                    false,
                );
            if let Err(e) = command
                .create_response(
                    &ctx.http,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new().embed(embed).ephemeral(true),
                    ),
                )
                .await
            {
                warn!(error = %e, "Failed to send review add response");
            }
        }
        Err(e) => {
            error!(error = %e, "Erreur ajout review");
            edit_response_text(ctx, command, &format!("Erreur : {e}")).await;
        }
    }
}

async fn handle_list(ctx: &Context, command: &CommandInteraction) {
    let guild_id = match command.guild_id {
        Some(id) => id,
        None => {
            edit_response_text(ctx, command, "Commande serveur uniquement.").await;
            return;
        }
    };

    let data = ctx.data.read().await;
    let api = match data.get::<ModerationApiKey>() {
        Some(a) => a,
        None => return,
    };

    let reviews = match api.list_pending_reviews(&guild_id.to_string()).await {
        Ok(v) => v,
        Err(e) => {
            edit_response_text(ctx, command, &format!("Erreur : {e}")).await;
            return;
        }
    };

    let description = if reviews.is_empty() {
        "Aucune review en attente.".to_string()
    } else {
        reviews
            .iter()
            .take(10)
            .enumerate()
            .map(|(i, r)| {
                let action_type = r.action_type.as_deref().unwrap_or("?");
                let target = r.target_name.as_deref().unwrap_or("?");
                let reason = r.reason.as_deref().unwrap_or("_sans raison_");
                format!(
                    "{}. `{}` — **{}** sur `{}` (par <@{}>)\n   Raison: _{}_",
                    i + 1,
                    short_id(&r.id),
                    action_type,
                    target,
                    r.added_by,
                    reason
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    };

    let embed = info_embed(format!(
        "\u{1f4cb} Reviews en attente ({})",
        reviews.len()
    ))
    .description(description);

    if let Err(e) = command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new().embed(embed).ephemeral(true),
            ),
        )
        .await
    {
        warn!(error = %e, "Failed to send review list response");
    }
}

async fn handle_resolve(
    ctx: &Context,
    command: &CommandInteraction,
    opts: &[serenity::model::application::CommandDataOption],
) {
    let review_id = opts
        .iter()
        .find(|o| o.name == "review_id")
        .and_then(|o| match &o.value {
            CommandDataOptionValue::String(s) => Some(s.as_str()),
            _ => None,
        })
        .unwrap_or("");
    let status = opts
        .iter()
        .find(|o| o.name == "status")
        .and_then(|o| match &o.value {
            CommandDataOptionValue::String(s) => Some(s.as_str()),
            _ => None,
        })
        .unwrap_or("");
    let notes = opts
        .iter()
        .find(|o| o.name == "notes")
        .and_then(|o| match &o.value {
            CommandDataOptionValue::String(s) => Some(s.as_str()),
            _ => None,
        });

    if review_id.is_empty() || status.is_empty() {
        edit_response_text(ctx, command, "review_id et status requis.").await;
        return;
    }

    let data = ctx.data.read().await;
    let api = match data.get::<ModerationApiKey>() {
        Some(a) => a,
        None => return,
    };

    api.resolve_review(
        review_id,
        status,
        &command.user.id.to_string(),
        &command.user.name,
        notes,
    )
    .await;

    let icon = match status {
        "approved" => "\u{2705}",
        "rejected" => "\u{274c}",
        "changed" => "\u{270f}\u{fe0f}",
        _ => "\u{2753}",
    };

    let embed = success_embed(format!("{} Review resolue — {}", icon, status))
        .field("Review", format!("`{}`", short_id(review_id)), true)
        .field("Par", format!("<@{}>", command.user.id), true);
    let embed = match notes {
        Some(n) if !n.is_empty() => embed.field("Notes", n, false),
        _ => embed,
    };

    if let Err(e) = command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new().embed(embed).ephemeral(true),
            ),
        )
        .await
    {
        warn!(error = %e, "Failed to send review resolve response");
    }
}

fn short_id(full: &str) -> String {
    full.chars().take(8).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_id_first_8_chars() {
        assert_eq!(short_id("0123456789abcdef"), "01234567");
    }
}
