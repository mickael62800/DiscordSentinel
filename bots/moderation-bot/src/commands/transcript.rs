//! MOD #8 — Commande `/transcript` : genere un transcript texte des derniers
//! messages d'un salon et l'envoie en piece jointe.
//!
//! Cas d'usage principal : apres une call room de moderation (voir `/call`),
//! le moderateur veut archiver la conversation pour dossier. Au lieu de
//! copier-coller manuellement, il lance `/transcript channel:<room>`.
//!
//! Design pragmatique : le bot fetch directement via serenity (deja Gateway+HTTP
//! authentifie), pas de round-trip vers `export-worker`. Le pattern queue async
//! reste disponible dans `export-worker` pour les exports massifs DB-based.
//!
//! Limites volontaires MVP :
//! - Max 100 messages (1 appel Discord, pas de pagination — suffisant pour
//!   la plupart des call rooms qui font < 50 messages)
//! - Format texte simple : `[YYYY-MM-DD HH:MM] Auteur: contenu`
//! - Attachments/embeds Discord ignores (juste un placeholder `[attachment]`)
//! - Taille max attachment Discord : 10 MB (free) / 25 MB (boost 2) — un
//!   transcript texte de 100 messages fait generalement < 50 KB, largement OK

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
            reply_text(ctx, command, "Salon requis.").await;
            return;
        }
    };

    // Defere immediatement : Discord a un timeout de 3s sur la reponse initiale,
    // et le fetch des messages peut prendre 1-2s.
    let defer = CreateInteractionResponse::Defer(
        CreateInteractionResponseMessage::new().ephemeral(true),
    );
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

/// Fetch les 100 derniers messages et serialise en texte brut.
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

    // Discord renvoie les messages du plus recent au plus ancien. On inverse.
    let mut lines: Vec<String> = Vec::with_capacity(messages.len() + 4);
    lines.push(format!(
        "=== Transcript du salon {} ===",
        channel_id
    ));
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
    // serenity::model::Timestamp -> Option<String> via to_rfc3339()
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

    // Annotations pour embeds/attachments (MVP minimal)
    if !msg.attachments.is_empty() {
        body.push_str(&format!(
            " [+{} piece(s) jointe(s)]",
            msg.attachments.len()
        ));
    }
    if !msg.embeds.is_empty() {
        body.push_str(&format!(" [+{} embed(s)]", msg.embeds.len()));
    }

    format!("[{timestamp}] {author}: {body}")
}

/// Estimation grossiere du nombre de lignes dans le transcript pour le
/// message de confirmation (sans re-parser le contenu).
fn count_lines_hint(bytes: usize) -> usize {
    // ~80 chars par message en moyenne + 4 lignes d'en-tete
    (bytes / 80).saturating_sub(4).max(1)
}

async fn reply_text(ctx: &Context, command: &CommandInteraction, content: &str) {
    if let Err(e) = command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content(content)
                    .ephemeral(true),
            ),
        )
        .await
    {
        warn!(error = %e, "Failed to send transcript reply");
    }
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
        // 320 bytes / 80 = 4, - 4 header = 0 -> max(1)
        assert_eq!(count_lines_hint(320), 1);
    }

    #[test]
    fn count_lines_hint_typical() {
        // ~8000 bytes / 80 = 100, - 4 = 96
        assert_eq!(count_lines_hint(8000), 96);
    }
}
