use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context, CreateCommand,
    CreateCommandOption, CreateInteractionResponse, CreateInteractionResponseMessage,
};
use tracing::error;

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
        None => { reply(ctx, command, "Commande serveur uniquement.").await; return; }
    };

    let data = ctx.data.read().await;
    let api = data.get::<ModerationApiKey>().unwrap();

    match api.get_history(&guild_id.to_string(), &target_id.to_string()).await {
        Ok(history) => {
            let mut msg = format!(
                "**Historique de <@{}>**\n🟡 Warns: {} | 🔇 Mutes: {} | 🔨 Bans: {}\n\n",
                target_id, history.total_warns, history.total_mutes, history.total_bans
            );

            if history.actions.is_empty() {
                msg.push_str("Aucune sanction enregistree.");
            } else {
                for (i, action) in history.actions.iter().take(10).enumerate() {
                    let emoji = match action.action_type.as_str() {
                        "warn" => "🟡",
                        "mute_temp" | "mute_permanent" => "🔇",
                        "ban_temp" | "ban_permanent" => "🔨",
                        "unmute" => "🔊",
                        "unban" => "✅",
                        _ => "📝",
                    };
                    msg.push_str(&format!(
                        "{}. {} **{}** — {}\n",
                        i + 1,
                        emoji,
                        action.action_type,
                        action.reason
                    ));
                }
            }

            reply(ctx, command, &msg).await;
        }
        Err(e) => {
            error!(error = %e, "Erreur recuperation historique");
            reply(ctx, command, &format!("Erreur : {e}")).await;
        }
    }
}

async fn reply(ctx: &Context, command: &CommandInteraction, content: &str) {
    command.create_response(
        &ctx.http,
        CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new().content(content).ephemeral(true),
        ),
    ).await.ok();
}
