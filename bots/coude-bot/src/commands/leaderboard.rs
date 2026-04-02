use serenity::all::{
    CommandInteraction, Context, CreateCommand, CreateEmbed, CreateEmbedFooter,
    CreateInteractionResponse, CreateInteractionResponseMessage,
};

use crate::db::LeaderboardEntry;
use crate::handler::GameDbKey;

pub fn register() -> CreateCommand {
    CreateCommand::new("leaderboard")
        .description("Classement Coup de Coude")
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    let guild_id = match command.guild_id {
        Some(id) => id.to_string(),
        None => {
            reply_ephemeral(ctx, command, "Commande serveur uniquement.").await;
            return;
        }
    };

    let data = ctx.data.read().await;
    let db = data.get::<GameDbKey>().unwrap();

    let richest = db.leaderboard_richest(&guild_id, 5).await.unwrap_or_default();
    let levels = db.leaderboard_level(&guild_id, 5).await.unwrap_or_default();
    let thieves = db.leaderboard_thieves(&guild_id, 5).await.unwrap_or_default();
    let cowards = db.leaderboard_cowards(&guild_id, 5).await.unwrap_or_default();
    let chaos = db.leaderboard_chaos(&guild_id, 5).await.unwrap_or_default();

    let embed = CreateEmbed::new()
        .title("\u{1f3c6} Classement Coup de Coude")
        .color(0xE67E22)
        .field(
            "\u{1fa99} Les plus riches",
            format_leaderboard(&richest, "coins"),
            false,
        )
        .field(
            "\u{2b50} Plus haut niveau",
            format_leaderboard(&levels, "niv."),
            false,
        )
        .field(
            "\u{1f5e1}\u{fe0f} Plus gros voleurs",
            format_leaderboard(&thieves, "voles"),
            false,
        )
        .field(
            "\u{1f414} Les plus laches",
            format_leaderboard(&cowards, "refus"),
            false,
        )
        .field(
            "\u{1f300} Rois du chaos",
            format_leaderboard(&chaos, "events"),
            false,
        )
        .footer(CreateEmbedFooter::new("Coup de Coude | Sentinel"))
        .timestamp(serenity::model::Timestamp::now());

    command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new().embed(embed),
            ),
        )
        .await
        .ok();
}

fn format_leaderboard(entries: &[LeaderboardEntry], unit: &str) -> String {
    if entries.is_empty() {
        return "Aucun joueur pour le moment.".to_string();
    }

    let medals = ["\u{1f947}", "\u{1f948}", "\u{1f949}", "4.", "5."];

    entries
        .iter()
        .enumerate()
        .map(|(i, e)| {
            let medal = medals.get(i).unwrap_or(&"  ");
            format!("{} <@{}> — **{}** {}", medal, e.user_id, e.value, unit)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

async fn reply_ephemeral(ctx: &Context, command: &CommandInteraction, content: &str) {
    command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content(content)
                    .ephemeral(true),
            ),
        )
        .await
        .ok();
}
