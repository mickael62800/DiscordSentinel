//! MOD #2 — Commande `/evidence` : attache / liste des preuves sur une action.
//!
//! Sous-commandes :
//!   - `/evidence add <action_id> <url> [description]` — attache une preuve
//!   - `/evidence list <action_id>` — liste les preuves d'une action

use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context, CreateCommand,
    CreateCommandOption, CreateInteractionResponse, CreateInteractionResponseMessage,
};
use tracing::{error, warn};

use sentinel_shared::embeds::{info_embed, success_embed};

use crate::handler::ModerationApiKey;

pub fn register() -> CreateCommand {
    CreateCommand::new("evidence")
        .description("Attacher ou lister des preuves sur une action de moderation")
        .default_member_permissions(serenity::all::Permissions::MODERATE_MEMBERS)
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "add",
                "Attacher une preuve a une action existante",
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
                CreateCommandOption::new(CommandOptionType::String, "url", "URL de la preuve")
                    .required(true),
            )
            .add_sub_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "description",
                    "Description optionnelle (max 500 chars)",
                ),
            ),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "list",
                "Lister les preuves d'une action",
            )
            .add_sub_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "action_id",
                    "ID de l'action (UUID)",
                )
                .required(true),
            ),
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
            reply_text(ctx, command, "Sous-commande manquante.").await;
            return;
        }
    };

    match sub_name {
        "add" => handle_add(ctx, command, sub_opts).await,
        "list" => handle_list(ctx, command, sub_opts).await,
        _ => reply_text(ctx, command, "Sous-commande inconnue.").await,
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
    let url = opts
        .iter()
        .find(|o| o.name == "url")
        .and_then(|o| match &o.value {
            CommandDataOptionValue::String(s) => Some(s.as_str()),
            _ => None,
        })
        .unwrap_or("");
    let description = opts
        .iter()
        .find(|o| o.name == "description")
        .and_then(|o| match &o.value {
            CommandDataOptionValue::String(s) => Some(s.as_str()),
            _ => None,
        });

    if action_id.is_empty() || url.is_empty() {
        reply_text(ctx, command, "action_id et url requis.").await;
        return;
    }

    let data = ctx.data.read().await;
    let api = match data.get::<ModerationApiKey>() {
        Some(a) => a,
        None => {
            error!("ModerationApiKey manquant");
            return;
        }
    };

    match api
        .add_evidence(
            action_id,
            url,
            description,
            &command.user.id.to_string(),
            &command.user.name,
        )
        .await
    {
        Ok(ev) => {
            let embed = success_embed("\u{1f4ce} Preuve attachee")
                .field("Action", format!("`{}`", short_id(&ev.action_id)), true)
                .field("URL", url, false)
                .field("Par", format!("<@{}>", ev.uploaded_by), true);
            if let Err(e) = command
                .create_response(
                    &ctx.http,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new().embed(embed).ephemeral(true),
                    ),
                )
                .await
            {
                warn!(error = %e, "Failed to send evidence add response");
            }
        }
        Err(e) => {
            error!(error = %e, "Erreur ajout evidence");
            reply_text(ctx, command, &format!("Erreur : {e}")).await;
        }
    }
}

async fn handle_list(
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

    if action_id.is_empty() {
        reply_text(ctx, command, "action_id requis.").await;
        return;
    }

    let data = ctx.data.read().await;
    let api = match data.get::<ModerationApiKey>() {
        Some(a) => a,
        None => return,
    };

    let evidences = match api.list_evidence(action_id).await {
        Ok(v) => v,
        Err(e) => {
            reply_text(ctx, command, &format!("Erreur : {e}")).await;
            return;
        }
    };

    let description = if evidences.is_empty() {
        "Aucune preuve attachee a cette action.".to_string()
    } else {
        evidences
            .iter()
            .take(10)
            .enumerate()
            .map(|(i, e)| {
                let desc = e
                    .description
                    .as_deref()
                    .unwrap_or("_pas de description_");
                format!(
                    "{}. [Lien]({}) — par <@{}>\n   _{}_",
                    i + 1,
                    e.url,
                    e.uploaded_by,
                    desc
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    let embed = info_embed(format!(
        "\u{1f4ce} Preuves — action `{}`",
        short_id(action_id)
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
        warn!(error = %e, "Failed to send evidence list response");
    }
}

fn short_id(full: &str) -> String {
    full.chars().take(8).collect()
}

async fn reply_text(ctx: &Context, command: &CommandInteraction, content: &str) {
    if let Err(e) = command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new().content(content).ephemeral(true),
            ),
        )
        .await
    {
        warn!(error = %e, "Failed to send evidence error");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_id_truncates_to_8() {
        assert_eq!(
            short_id("12345678-abcd-ef00-1234-567890abcdef"),
            "12345678"
        );
    }

    #[test]
    fn short_id_shorter_unchanged() {
        assert_eq!(short_id("abc"), "abc");
    }
}
