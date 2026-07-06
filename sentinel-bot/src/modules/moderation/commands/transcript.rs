use crate::shared::discord_helpers::edit_response_text;
use chrono::Utc;
use serenity::all::{
    ChannelId, CommandDataOptionValue, CommandInteraction, CommandOptionType, Context,
    CreateAttachment, CreateCommand, CreateCommandOption, CreateInteractionResponse,
    CreateInteractionResponseFollowup, CreateInteractionResponseMessage, GetMessages,
};
use tracing::{error, warn};

pub fn register() -> CreateCommand {
    CreateCommand::new("transcript")
        .description("Genere un transcript texte des 100 derniers messages d'un salon")
        .default_member_permissions(serenity::all::Permissions::MODERATE_MEMBERS)
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::Channel,
                "channel",
                "Salon a transcrire (texte ou voix avec texte)",
            )
            .required(true),
        )
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    let channel_id = command
        .data
        .options
        .iter()
        .find(|o| o.name == "channel")
        .and_then(|o| match &o.value {
            CommandDataOptionValue::Channel(id) => Some(*id),
            _ => None,
        });

    let channel_id = match channel_id {
        Some(id) => id,
        None => {
            edit_response_text(ctx, command, "Salon requis.").await;
            return;
        }
    };

    // Garde de permission (F-2) : reserve aux modos (le default_member_permissions
    // est reecrivable par un admin de guilde).
    if !super::has_mod_permission(command, serenity::all::Permissions::MODERATE_MEMBERS) {
        edit_response_text(ctx, command, "❌ Permission de modération requise.").await;
        return;
    }
    // F-1 : l'invocateur doit avoir acces (VIEW_CHANNEL) au salon cible, sinon on
    // exposerait un salon prive/staff auquel il n'a pas droit.
    if let (Some(gid), Some(member)) = (command.guild_id, command.member.as_ref()) {
        let can_view = ctx
            .cache
            .guild(gid)
            .map(|g| match g.channels.get(&channel_id) {
                Some(ch) => g
                    .user_permissions_in(ch, member)
                    .contains(serenity::all::Permissions::VIEW_CHANNEL),
                None => false,
            })
            .unwrap_or(false);
        if !can_view {
            edit_response_text(ctx, command, "❌ Tu n'as pas accès à ce salon.").await;
            return;
        }
    }

    let defer =
        CreateInteractionResponse::Defer(CreateInteractionResponseMessage::new().ephemeral(true));
    if let Err(e) = command.create_response(&ctx.http, defer).await {
        warn!(error = %e, "Failed to defer transcript response");
        return;
    }

    let result = build_transcript(ctx, channel_id).await;

    match result {
        Ok((filename, content)) => {
            let size = content.len();
            let attachment = CreateAttachment::bytes(content.into_bytes(), filename.clone());
            let followup = CreateInteractionResponseFollowup::new()
                .content(format!(
                    "\u{1f4c4} Transcript genere ({} messages, {} ko) — voir piece jointe.",
                    count_lines_hint(size),
                    size / 1024 + 1
                ))
                .add_file(attachment)
                .ephemeral(true);

            if let Err(e) = command.create_followup(&ctx.http, followup).await {
                error!(error = %e, "Failed to send transcript followup");
            }
        }
        Err(msg) => {
            let followup = CreateInteractionResponseFollowup::new()
                .content(format!("\u{274c} Erreur : {msg}"))
                .ephemeral(true);
            if let Err(e) = command.create_followup(&ctx.http, followup).await {
                warn!(error = %e, "Failed to send transcript error followup");
            }
        }
    }
}

async fn build_transcript(
    ctx: &Context,
    channel_id: ChannelId,
) -> Result<(String, String), String> {
    let messages = channel_id
        .messages(&ctx.http, GetMessages::new().limit(100))
        .await
        .map_err(|e| format!("impossible de lire le salon : {e}"))?;

    if messages.is_empty() {
        return Err("aucun message dans ce salon".to_string());
    }

    let mut lines: Vec<String> = Vec::with_capacity(messages.len() + 4);
    lines.push(format!("=== Transcript du salon {} ===", channel_id));
    lines.push(format!(
        "Genere le {} UTC",
        Utc::now().format("%Y-%m-%d %H:%M:%S")
    ));
    lines.push(format!("Nombre de messages : {}", messages.len()));
    lines.push(String::new());

    for msg in messages.iter().rev() {
        lines.push(format_message(msg));
    }

    let content = lines.join("\n");
    let filename = format!(
        "transcript-{}-{}.txt",
        channel_id,
        Utc::now().format("%Y%m%d-%H%M%S")
    );
    Ok((filename, content))
}

fn format_message(msg: &serenity::model::channel::Message) -> String {
    let timestamp = msg
        .timestamp
        .to_rfc3339()
        .unwrap_or_default()
        .replace('T', " ")
        .chars()
        .take(19)
        .collect::<String>();

    let author = &msg.author.name;

    let mut body = if msg.content.is_empty() {
        "[message vide]".to_string()
    } else {
        msg.content.clone()
    };

    if !msg.attachments.is_empty() {
        body.push_str(&format!(" [+{} piece(s) jointe(s)]", msg.attachments.len()));
    }
    if !msg.embeds.is_empty() {
        body.push_str(&format!(" [+{} embed(s)]", msg.embeds.len()));
    }

    format!("[{timestamp}] {author}: {body}")
}

fn count_lines_hint(bytes: usize) -> usize {
    (bytes / 80).saturating_sub(4).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn count_lines_hint_zero() {
        assert_eq!(count_lines_hint(0), 1);
    }

    #[test]
    fn count_lines_hint_small() {
        assert_eq!(count_lines_hint(320), 1);
    }

    #[test]
    fn count_lines_hint_typical() {
        assert_eq!(count_lines_hint(8000), 96);
    }
}
