use serenity::all::{
    CommandInteraction, Context, CreateCommand, CreateInteractionResponse,
    CreateInteractionResponseMessage,
};
use tracing::{error, warn};

use crate::shared::embeds::info_embed;

use super::api_client::ModStatsEntry;
use super::ModerationApiKey;
use crate::shared::discord_helpers::edit_response_text;

pub fn register() -> CreateCommand {
    CreateCommand::new("modstats")
        .description("Metriques d'activite des moderateurs (30 derniers jours)")
        .default_member_permissions(serenity::all::Permissions::MODERATE_MEMBERS)
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    if !super::has_mod_permission(command, serenity::all::Permissions::MODERATE_MEMBERS) {
        let _ = command
            .create_response(
                &ctx.http,
                serenity::all::CreateInteractionResponse::Message(
                    serenity::all::CreateInteractionResponseMessage::new()
                        .content("❌ Permission de modération requise.")
                        .ephemeral(true),
                ),
            )
            .await;
        return;
    }
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

    let stats = match api.get_modstats(&guild_id.to_string()).await {
        Ok(s) => s,
        Err(e) => {
            error!(error = %e, "Erreur recuperation modstats");
            edit_response_text(ctx, command, &format!("Erreur : {e}")).await;
            return;
        }
    };

    let description = format_modstats(&stats);

    let embed = info_embed("\u{1f4ca} Statistiques de moderation — 30 derniers jours")
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
        warn!(error = %e, "Failed to send modstats response");
    }
}

fn format_modstats(stats: &[ModStatsEntry]) -> String {
    if stats.is_empty() {
        return "Aucune action de moderation enregistree ces 30 derniers jours.".to_string();
    }

    let mut lines = Vec::with_capacity(stats.len());
    for (i, entry) in stats.iter().enumerate() {
        let medal = match i {
            0 => "\u{1f947}",
            1 => "\u{1f948}",
            2 => "\u{1f949}",
            _ => "\u{1f538}",
        };
        lines.push(format!(
            "{} **{}** — {} actions\n   \u{26a0}\u{fe0f} {} · \u{1f507} {} · \u{1f6ab} {} · \u{1f462} {}",
            medal,
            entry.moderator_name,
            entry.total,
            entry.warns,
            entry.mutes,
            entry.bans,
            entry.kicks,
        ));
    }

    lines.join("\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk(name: &str, total: i64, w: i64, m: i64, b: i64, k: i64) -> ModStatsEntry {
        ModStatsEntry {
            moderator_id: "1".into(),
            moderator_name: name.into(),
            total,
            warns: w,
            mutes: m,
            bans: b,
            kicks: k,
        }
    }

    #[test]
    fn empty_list_returns_placeholder() {
        assert_eq!(
            format_modstats(&[]),
            "Aucune action de moderation enregistree ces 30 derniers jours."
        );
    }

    #[test]
    fn single_entry_has_gold_medal() {
        let stats = vec![mk("alice", 5, 2, 1, 1, 1)];
        let out = format_modstats(&stats);
        assert!(out.contains("\u{1f947}"));
        assert!(out.contains("alice"));
        assert!(out.contains("5 actions"));
    }

    #[test]
    fn top_three_have_medals() {
        let stats = vec![
            mk("alice", 10, 5, 3, 2, 0),
            mk("bob", 8, 4, 2, 2, 0),
            mk("carol", 5, 3, 1, 1, 0),
            mk("dan", 2, 1, 1, 0, 0),
        ];
        let out = format_modstats(&stats);
        assert!(out.contains("\u{1f947}"));
        assert!(out.contains("\u{1f948}"));
        assert!(out.contains("\u{1f949}"));
        assert!(out.contains("\u{1f538}"));
    }
}
