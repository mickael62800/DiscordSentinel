use serenity::all::{
    ButtonStyle, CommandInteraction, ComponentInteraction, Context, CreateActionRow, CreateButton,
    CreateCommand, CreateEmbed, CreateEmbedFooter, CreateInteractionResponse,
    CreateInteractionResponseMessage,
};

use crate::shared::discord_helpers::{reply_ephemeral, require_guild_id};

use crate::modules::coude::api_client::LeaderboardEntry;
use crate::modules::coude::GameApiKey;
use crate::modules::coude::load_guild_config;

/// custom_id du bouton "Mettre a jour" du panneau de classement. Le panneau
/// est un message unique persistant : le bouton edite ce meme message avec les
/// donnees fraiches (pas de nouveau message, pas besoin de relancer la commande).
pub const REFRESH_ID: &str = "coude_lb_refresh";

pub fn register() -> CreateCommand {
    CreateCommand::new("leaderboard")
        .description("Affiche le panneau de classement Coup de Coude (avec bouton Mettre a jour)")
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    let Some(guild_id) = require_guild_id(ctx, command).await else { return; };

    let config = load_guild_config(ctx, &guild_id).await;
    if !crate::modules::coude::channel_check::check_channel(ctx, command, config.channel_leaderboard()).await {
        return;
    }
    if !config.enabled() {
        reply_ephemeral(ctx, command, "Le jeu Coup de Coude est desactive sur ce serveur.").await;
        return;
    }

    let embed = build_leaderboard_embed(ctx, &guild_id).await;

    if let Err(e) = command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .embed(embed)
                    .components(vec![refresh_row()]),
            ),
        )
        .await
    {
        tracing::warn!(error = %e, "Echec response Discord");
    }
}

/// Handler du bouton "Mettre a jour" : reconstruit le classement et edite le
/// message du panneau en place (UpdateMessage). N'importe qui peut rafraichir.
pub async fn handle_refresh(ctx: &Context, component: &ComponentInteraction) {
    let guild_id = match component.guild_id {
        Some(g) => g.to_string(),
        None => return,
    };

    let embed = build_leaderboard_embed(ctx, &guild_id).await;

    if let Err(e) = component
        .create_response(
            &ctx.http,
            CreateInteractionResponse::UpdateMessage(
                CreateInteractionResponseMessage::new()
                    .embed(embed)
                    .components(vec![refresh_row()]),
            ),
        )
        .await
    {
        tracing::warn!(error = %e, "Echec update panneau leaderboard");
    }
}

/// Construit l'embed de classement (5 categories). Partage entre la commande
/// (pose du panneau) et le bouton (rafraichissement).
async fn build_leaderboard_embed(ctx: &Context, guild_id: &str) -> CreateEmbed {
    let data = ctx.data.read().await;
    let api = data.get::<GameApiKey>().unwrap();

    let richest = api.leaderboard_richest(guild_id, 5).await.unwrap_or_else(|e| {
        tracing::warn!(error = %e, "Echec API leaderboard_richest");
        vec![]
    });
    let levels = api.leaderboard_level(guild_id, 5).await.unwrap_or_else(|e| {
        tracing::warn!(error = %e, "Echec API leaderboard_level");
        vec![]
    });
    let thieves = api.leaderboard_thieves(guild_id, 5).await.unwrap_or_else(|e| {
        tracing::warn!(error = %e, "Echec API leaderboard_thieves");
        vec![]
    });
    let cowards = api.leaderboard_cowards(guild_id, 5).await.unwrap_or_else(|e| {
        tracing::warn!(error = %e, "Echec API leaderboard_cowards");
        vec![]
    });
    let chaos = api.leaderboard_chaos(guild_id, 5).await.unwrap_or_else(|e| {
        tracing::warn!(error = %e, "Echec API leaderboard_chaos");
        vec![]
    });

    CreateEmbed::new()
        .title("\u{1f3c6} Classement Coup de Coude")
        .color(0xE67E22)
        .field("\u{1fa99} Les plus riches", format_leaderboard(&richest, "coins"), false)
        .field("\u{2b50} Plus haut niveau", format_leaderboard(&levels, "niv."), false)
        .field("\u{1f5e1}\u{fe0f} Plus gros voleurs", format_leaderboard(&thieves, "voles"), false)
        .field("\u{1f414} Les plus laches", format_leaderboard(&cowards, "refus"), false)
        .field("\u{1f300} Rois du chaos", format_leaderboard(&chaos, "events"), false)
        .footer(CreateEmbedFooter::new(format!(
            "{} — clique sur Mettre a jour pour rafraichir",
            crate::shared::branding::COUDE_TAGLINE_SHORT
        )))
        .timestamp(serenity::model::Timestamp::now())
}

/// Row contenant le bouton "Mettre a jour".
fn refresh_row() -> CreateActionRow {
    CreateActionRow::Buttons(vec![
        CreateButton::new(REFRESH_ID)
            .label("Mettre a jour")
            .emoji(serenity::model::channel::ReactionType::Unicode("\u{1f504}".into()))
            .style(ButtonStyle::Primary),
    ])
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
