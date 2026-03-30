use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context, CreateCommand,
    CreateCommandOption, CreateInteractionResponse, CreateInteractionResponseMessage,
    CreateMessage,
};
use tracing::{error, info};

use sentinel_shared::embeds::{sentinel_embed, gravity_color, gravity_emoji};

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
        None => { reply_text(ctx, command, "Commande serveur uniquement.").await; return; }
    };

    let target = match target_id.to_user(&ctx.http).await {
        Ok(u) => u,
        Err(_) => { reply_text(ctx, command, "Utilisateur introuvable.").await; return; }
    };

    // Log dans le backend
    let data = ctx.data.read().await;
    let api = data.get::<ModerationApiKey>().unwrap();

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

            let guild_name = guild_id.to_partial_guild(&ctx.http).await
                .map(|g| g.name).unwrap_or_else(|_| "le serveur".into());

            // DM a l'utilisateur
            if let Ok(dm) = target.create_dm_channel(&ctx.http).await {
                let dm_embed = sentinel_embed(
                    format!("{} Avertissement sur **{guild_name}**", gravity_emoji(gravity)),
                    gravity_color(gravity),
                )
                .field("Gravite", gravity, true)
                .field("Raison", reason, false);

                dm.send_message(
                    &ctx.http,
                    CreateMessage::new().embed(dm_embed),
                ).await.ok();
            }

            let channel_embed = sentinel_embed(
                format!("{} Warn ({gravity})", gravity_emoji(gravity)),
                gravity_color(gravity),
            )
            .field("Cible", format!("<@{}>", target.id), true)
            .field("Moderateur", format!("<@{}>", command.user.id), true)
            .field("Gravite", gravity, true)
            .field("Raison", reason, false);

            command.create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new().embed(channel_embed),
                ),
            ).await.ok();
        }
        Err(e) => {
            error!(error = %e, "Erreur log warn");
            reply_text(ctx, command, &format!("Erreur : {e}")).await;
        }
    }
}

async fn reply_text(ctx: &Context, command: &CommandInteraction, content: &str) {
    command.create_response(
        &ctx.http,
        CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new().content(content).ephemeral(false),
        ),
    ).await.ok();
}
