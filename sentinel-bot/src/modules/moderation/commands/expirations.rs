use chrono::{DateTime, Utc};
use serenity::all::{
    CommandInteraction, Context, CreateCommand, CreateInteractionResponse,
    CreateInteractionResponseMessage,
};
use tracing::{error, warn};

use crate::shared::embeds::info_embed;

use super::ModerationApiKey;
use crate::shared::discord_helpers::edit_response_text;

pub fn register() -> CreateCommand {
    CreateCommand::new("expirations")
        .description("Liste les sanctions temporaires actives avec leur temps restant")
        .default_member_permissions(serenity::all::Permissions::MODERATE_MEMBERS)
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
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
        None => {
            error!("ModerationApiKey manquant");
            return;
        }
    };

    let reminders = match api.get_active_reminders(&guild_id.to_string()).await {
        Ok(r) => r,
        Err(e) => {
            error!(error = %e, "Erreur recuperation reminders");
            edit_response_text(ctx, command, &format!("Erreur : {e}")).await;
            return;
        }
    };

    let now = Utc::now();
    let mut active: Vec<_> = reminders
        .into_iter()
        .filter_map(|r| {
            let expires_at = DateTime::parse_from_rfc3339(&r.expires_at).ok()?;
            let expires_at_utc = expires_at.with_timezone(&Utc);
            if expires_at_utc <= now {
                return None;
            }
            Some((r, expires_at_utc))
        })
        .collect();

    active.sort_by_key(|(_, exp)| *exp);

    let description = if active.is_empty() {
        "Aucune sanction temporaire active.".to_string()
    } else {
        active
            .iter()
            .take(15)
            .enumerate()
            .map(|(i, (r, exp))| {
                let icon = match r.action_type.as_str() {
                    "mute_temp" => "\u{1f507}",
                    "ban_temp" => "\u{1f6ab}",
                    _ => "\u{23f3}",
                };
                let remaining = *exp - now;
                let remaining_str = format_duration(remaining);
                let reason_trunc: String = r.reason.chars().take(60).collect();
                format!(
                    "{}. {} <@{}> — **{}** — expire dans **{}** (par {})\n   _{}_",
                    i + 1,
                    icon,
                    r.target_id,
                    r.action_type,
                    remaining_str,
                    r.moderator_name,
                    reason_trunc
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    let total = active.len();
    let embed = info_embed(format!(
        "\u{23f0} Sanctions temporaires actives ({})",
        total
    ))
    .description(description);

    if let Err(e) = command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .embed(embed)
                    .ephemeral(true),
            ),
        )
        .await
    {
        warn!(error = %e, "Failed to send expirations response");
    }
}

fn format_duration(d: chrono::Duration) -> String {
    let total_secs = d.num_seconds().max(0);
    let days = total_secs / 86_400;
    let hours = (total_secs % 86_400) / 3_600;
    let minutes = (total_secs % 3_600) / 60;

    if days > 0 {
        format!("{}j {}h", days, hours)
    } else if hours > 0 {
        format!("{}h {}min", hours, minutes)
    } else {
        format!("{} min", minutes.max(1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_duration_minutes() {
        let d = chrono::Duration::minutes(45);
        assert_eq!(format_duration(d), "45 min");
    }

    #[test]
    fn format_duration_hours() {
        let d = chrono::Duration::minutes(3 * 60 + 20);
        assert_eq!(format_duration(d), "3h 20min");
    }

    #[test]
    fn format_duration_days() {
        let d = chrono::Duration::hours(2 * 24 + 5);
        assert_eq!(format_duration(d), "2j 5h");
    }

    #[test]
    fn format_duration_minimum_one_minute() {
        let d = chrono::Duration::seconds(20);
        assert_eq!(format_duration(d), "1 min");
    }

    #[test]
    fn format_duration_negative_is_zero() {
        let d = chrono::Duration::seconds(-100);
        assert_eq!(format_duration(d), "1 min");
    }
}
