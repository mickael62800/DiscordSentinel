use serenity::all::{
    CommandInteraction, CommandOptionType, Context, CreateCommand,
    CreateCommandOption,
};

use sentinel_shared::discord_helpers::{reply_ephemeral, reply_ephemeral_embed};
use sentinel_shared::embeds::info_embed;

use crate::detectors::{self, DetectorConfig};
use crate::handler::{FloodTrackerKey, ProcessedMessagesKey, SlowmodeTrackerKey};

/// Check permission serveur : default_member_permissions est juste un hint
/// UI Discord (override par les params de guild ou une interaction forgee).
/// On revalide explicitement MANAGE_GUILD sur chaque call.
fn has_manage_guild(command: &CommandInteraction) -> bool {
    command
        .member
        .as_ref()
        .and_then(|m| m.permissions)
        .map(|p| {
            p.contains(serenity::all::Permissions::MANAGE_GUILD)
                || p.contains(serenity::all::Permissions::ADMINISTRATOR)
        })
        .unwrap_or(false)
}

pub fn register() -> CreateCommand {
    CreateCommand::new("automod")
        .description("Commandes de l'automod bot")
        .default_member_permissions(serenity::all::Permissions::MANAGE_GUILD)
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "status",
                "Affiche l'etat actuel de l'automod",
            ),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "test",
                "Teste l'analyse d'un message",
            )
            .add_sub_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "message",
                    "Message a analyser",
                )
                .required(true),
            ),
        )
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    // Check permission serveur avant tout. Meme si /automod est read-only,
    // expose des infos internes (tailles de caches, etat des trackers)
    // qui ne doivent pas etre accessibles a n'importe qui.
    if !has_manage_guild(command) {
        reply_ephemeral(
            ctx,
            command,
            "❌ Permission MANAGE_GUILD requise pour /automod.",
        )
        .await;
        tracing::warn!(
            user = %command.user.name,
            user_id = %command.user.id,
            "Tentative /automod sans permission"
        );
        return;
    }

    let sub = command
        .data
        .options
        .first()
        .map(|o| o.name.as_str())
        .unwrap_or("");

    match sub {
        "status" => handle_status(ctx, command).await,
        "test" => handle_test(ctx, command).await,
        _ => {}
    }
}

async fn handle_status(ctx: &Context, command: &CommandInteraction) {
    let data = ctx.data.read().await;

    let processed_count = data
        .get::<ProcessedMessagesKey>()
        .map(|s| s.len())
        .unwrap_or(0);

    let flood_count = data
        .get::<FloodTrackerKey>()
        .map(|m| m.len())
        .unwrap_or(0);

    let slowmode_channels = data
        .get::<SlowmodeTrackerKey>()
        .map(|t| t.tracked_channels())
        .unwrap_or(0);

    let embed = info_embed("Automod — Statut")
        .field("Messages traites (cache)", processed_count.to_string(), true)
        .field("Flood trackers actifs", flood_count.to_string(), true)
        .field("Channels slowmode", slowmode_channels.to_string(), true);

    reply_ephemeral_embed(ctx, command, embed).await;
}

async fn handle_test(ctx: &Context, command: &CommandInteraction) {
    let message = command
        .data
        .options
        .first()
        .and_then(|sub| {
            sub.value
                .as_str()
                .map(|s| s.to_string())
                .or_else(|| {
                    // SubCommand: options are nested
                    use serenity::all::CommandDataOptionValue;
                    if let CommandDataOptionValue::SubCommand(opts) = &sub.value {
                        opts.iter()
                            .find(|o| o.name == "message")
                            .and_then(|o| o.value.as_str().map(|s| s.to_string()))
                    } else {
                        None
                    }
                })
        })
        .unwrap_or_default();

    let flags = detectors::analyze(&message, &DetectorConfig::default());

    let preview = if message.chars().count() > 100 {
        let truncated: String = message.chars().take(100).collect();
        format!("{}...", truncated)
    } else {
        message.clone()
    };

    let embed = info_embed("Automod — Test d'analyse")
        .field("Message", format!("```{}```", preview), false)
        .field("Spam", if flags.spam { "Oui" } else { "Non" }, true)
        .field("Insulte", if flags.insult { "Oui" } else { "Non" }, true)
        .field("Lien", if flags.link { "Oui" } else { "Non" }, true)
        .field("Phishing", if flags.phishing { "Oui" } else { "Non" }, true);

    reply_ephemeral_embed(ctx, command, embed).await;
}
