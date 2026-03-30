use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context, CreateCommand,
    CreateCommandOption, CreateInteractionResponse, CreateInteractionResponseMessage,
};
use tracing::{error, info};

use crate::api_client::ModerationAction;
use crate::handler::ModerationApiKey;

pub fn register() -> CreateCommand {
    CreateCommand::new("warn")
        .description("Avertir un utilisateur")
        .add_option(
            CreateCommandOption::new(CommandOptionType::User, "user", "Utilisateur a avertir")
                .required(true),
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::String, "gravity", "Gravite de l'avertissement")
                .required(true)
                .add_string_choice("Faible", "low")
                .add_string_choice("Moyenne", "medium")
                .add_string_choice("Haute", "high"),
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::String, "reason", "Raison de l'avertissement")
                .required(true),
        )
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    let options = &command.data.options;

    let target_id = options.iter().find(|o| o.name == "user")
        .and_then(|o| match &o.value { CommandDataOptionValue::User(id) => Some(*id), _ => None })
        .unwrap();

    let gravity = options.iter().find(|o| o.name == "gravity")
        .and_then(|o| match &o.value { CommandDataOptionValue::String(s) => Some(s.as_str()), _ => None })
        .unwrap_or("medium");

    let reason = options.iter().find(|o| o.name == "reason")
        .and_then(|o| match &o.value { CommandDataOptionValue::String(s) => Some(s.as_str()), _ => None })
        .unwrap_or("Aucune raison");

    let guild_id = match command.guild_id {
        Some(id) => id,
        None => { reply(ctx, command, "Commande serveur uniquement.").await; return; }
    };

    let target = match target_id.to_user(&ctx.http).await {
        Ok(u) => u,
        Err(_) => { reply(ctx, command, "Utilisateur introuvable.").await; return; }
    };

    // Log dans le backend
    let data = ctx.data.read().await;
    let api = data.get::<ModerationApiKey>().unwrap();

    let gravity_emoji = match gravity {
        "low" => "🟡",
        "medium" => "🟠",
        "high" => "🔴",
        _ => "⚪",
    };

    let action = ModerationAction {
        guild_id: guild_id.to_string(),
        channel_id: command.channel_id.to_string(),
        moderator_id: command.user.id.to_string(),
        moderator_name: command.user.name.clone(),
        target_id: target.id.to_string(),
        target_name: target.name.clone(),
        action_type: "warn".to_string(),
        reason: reason.to_string(),
        gravity: Some(gravity.to_string()),
        duration: None,
    };

    match api.log_action(&action).await {
        Ok(resp) => {
            info!(
                action_id = %resp.id,
                target = %target.name,
                gravity = gravity,
                "Warn enregistre"
            );

            // DM a l'utilisateur
            if let Ok(dm) = target.create_dm_channel(&ctx.http).await {
                dm.send_message(
                    &ctx.http,
                    serenity::builder::CreateMessage::new().content(format!(
                        "{gravity_emoji} **Avertissement ({gravity})** sur **{}**\nRaison : {reason}",
                        guild_id.to_partial_guild(&ctx.http).await
                            .map(|g| g.name).unwrap_or_else(|_| "le serveur".into()),
                    )),
                ).await.ok();
            }

            reply(ctx, command, &format!(
                "{gravity_emoji} **Warn ({gravity})** applique a <@{}>.\nRaison : {reason}",
                target.id
            )).await;
        }
        Err(e) => {
            error!(error = %e, "Erreur log warn");
            reply(ctx, command, &format!("Erreur : {e}")).await;
        }
    }
}

async fn reply(ctx: &Context, command: &CommandInteraction, content: &str) {
    command.create_response(
        &ctx.http,
        CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new().content(content).ephemeral(false),
        ),
    ).await.ok();
}
