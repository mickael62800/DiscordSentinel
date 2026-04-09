use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context, CreateCommand,
    CreateCommandOption, CreateInteractionResponse, CreateInteractionResponseMessage,
};
use tracing::{error, info, warn};

use sentinel_shared::embeds::success_embed;

use crate::handler::ModerationApiKey;

pub fn register() -> CreateCommand {
    CreateCommand::new("note")
        .description("Ajouter une note interne sur un utilisateur")
        .default_member_permissions(serenity::all::Permissions::MODERATE_MEMBERS)
        .add_option(
            CreateCommandOption::new(CommandOptionType::User, "user", "Utilisateur concerne")
                .required(true),
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::String, "content", "Contenu de la note")
                .required(true),
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::String, "category", "Categorie de la note")
                .add_string_choice("General", "general")
                .add_string_choice("Avertissement", "warning")
                .add_string_choice("Positif", "positive")
                .add_string_choice("Contexte", "context"),
        )
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    let options = &command.data.options;

    let target_id = options.iter().find(|o| o.name == "user")
        .and_then(|o| match &o.value { CommandDataOptionValue::User(id) => Some(*id), _ => None })
        .unwrap();

    let content = options.iter().find(|o| o.name == "content")
        .and_then(|o| match &o.value { CommandDataOptionValue::String(s) => Some(s.as_str()), _ => None })
        .unwrap_or("");

    let category = options.iter().find(|o| o.name == "category")
        .and_then(|o| match &o.value { CommandDataOptionValue::String(s) => Some(s.as_str()), _ => None })
        .unwrap_or("general");

    let guild_id = match command.guild_id {
        Some(id) => id,
        None => { reply_text(ctx, command, "Commande serveur uniquement.").await; return; }
    };

    let target = match target_id.to_user(&ctx.http).await {
        Ok(u) => u,
        Err(_) => { reply_text(ctx, command, "Utilisateur introuvable.").await; return; }
    };

    let data = ctx.data.read().await;
    let api = match data.get::<ModerationApiKey>() {
        Some(a) => a,
        None => { tracing::error!("ModerationApiKey manquant"); return; }
    };

    match api.add_note(
        &guild_id.to_string(),
        &target.id.to_string(),
        &command.user.id.to_string(),
        &command.user.name,
        content,
        category,
    ).await {
        Ok(_) => {
            info!(target = %target.name, category = category, "Note ajoutee");

            let category_emoji = match category {
                "warning" => "\u{26a0}\u{fe0f}",
                "positive" => "\u{2705}",
                "context" => "\u{1f4cb}",
                _ => "\u{1f4dd}",
            };

            let embed = success_embed(format!("{category_emoji} Note ajoutee"))
                .field("Cible", format!("<@{}>", target.id), true)
                .field("Categorie", category, true)
                .field("Contenu", content, false);

            if let Err(e) = command.create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new().embed(embed).ephemeral(true),
                ),
            ).await {
                warn!(error = %e, "Failed to send note response");
            }
        }
        Err(e) => {
            error!(error = %e, "Erreur ajout note");
            reply_text(ctx, command, &format!("Erreur : {e}")).await;
        }
    }
}

async fn reply_text(ctx: &Context, command: &CommandInteraction, content: &str) {
    if let Err(e) = command.create_response(
        &ctx.http,
        CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new().content(content).ephemeral(true),
        ),
    ).await {
        warn!(error = %e, "Failed to send reply text");
    }
}
