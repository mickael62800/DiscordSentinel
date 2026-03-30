use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context, CreateCommand,
    CreateCommandOption, CreateInteractionResponse, CreateInteractionResponseMessage,
};
use tracing::error;

use sentinel_shared::embeds::{info_embed, action_emoji};

use crate::handler::ModerationApiKey;

pub fn register() -> CreateCommand {
    CreateCommand::new("history")
        .description("Voir l'historique des sanctions d'un utilisateur")
        .add_option(
            CreateCommandOption::new(CommandOptionType::User, "user", "Utilisateur a verifier")
                .required(true),
        )
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    let target_id = command.data.options.iter().find(|o| o.name == "user")
        .and_then(|o| match &o.value { CommandDataOptionValue::User(id) => Some(*id), _ => None })
        .unwrap();

    let guild_id = match command.guild_id {
        Some(id) => id,
        None => { reply_text(ctx, command, "Commande serveur uniquement.").await; return; }
    };

    let target = target_id.to_user(&ctx.http).await.ok();
    let username = target.as_ref().map(|u| u.name.as_str()).unwrap_or("inconnu");

    let data = ctx.data.read().await;
    let api = data.get::<ModerationApiKey>().unwrap();

    match api.get_history(&guild_id.to_string(), &target_id.to_string()).await {
        Ok(history) => {
            let description = if history.actions.is_empty() {
                "Aucune sanction enregistree.".to_string()
            } else {
                history.actions.iter().take(10).enumerate().map(|(i, action)| {
                    format!(
                        "{}. {} **{}** — {}",
                        i + 1,
                        action_emoji(&action.action_type),
                        action.action_type,
                        action.reason
                    )
                }).collect::<Vec<_>>().join("\n")
            };

            let embed = info_embed(format!("📋 Historique de @{username}"))
                .description(description)
                .field("Warns", history.total_warns.to_string(), true)
                .field("Mutes", history.total_mutes.to_string(), true)
                .field("Bans", history.total_bans.to_string(), true);

            command.create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new().embed(embed).ephemeral(true),
                ),
            ).await.ok();
        }
        Err(e) => {
            error!(error = %e, "Erreur recuperation historique");
            reply_text(ctx, command, &format!("Erreur : {e}")).await;
        }
    }
}

async fn reply_text(ctx: &Context, command: &CommandInteraction, content: &str) {
    command.create_response(
        &ctx.http,
        CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new().content(content).ephemeral(true),
        ),
    ).await.ok();
}
