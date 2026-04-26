use serenity::all::{
    CommandInteraction, Context, CreateCommand, CreateEmbed, CreateEmbedFooter,
    CreateInteractionResponse, CreateInteractionResponseMessage,
};

use sentinel_shared::discord_helpers::reply_ephemeral;

use crate::modules::coude::load_guild_config;
use crate::modules::coude::GameApiKey;

pub fn register() -> CreateCommand {
    CreateCommand::new("saison")
        .description("Affiche les infos de la saison en cours")
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    let guild_id = match command.guild_id {
        Some(id) => id.to_string(),
        None => {
            reply_ephemeral(ctx, command, "Commande serveur uniquement.").await;
            return;
        }
    };

    let config = load_guild_config(ctx, &guild_id).await;
    if !crate::modules::coude::channel_check::check_channel(ctx, command, config.channel_profil()).await {
        return;
    }

    let data = ctx.data.read().await;
    let api = match data.get::<GameApiKey>() {
        Some(a) => a,
        None => return,
    };

    // Recuperer la saison en cours (l'API la cree automatiquement si elle n'existe pas)
    let season = api.get_current_season(&guild_id).await.ok();

    // Recuperer les top joueurs pour le classement saisonnier
    let top_coins = api.leaderboard_richest(&guild_id, 3).await.unwrap_or_default();
    let top_level = api.leaderboard_level(&guild_id, 3).await.unwrap_or_default();
    let top_thieves = api.leaderboard_thieves(&guild_id, 3).await.unwrap_or_default();

    let medals = ["\u{1f947}", "\u{1f948}", "\u{1f949}"];

    let coins_ranking = top_coins.iter().enumerate()
        .map(|(i, e)| format!("{} **{}** — {} coins", medals.get(i).unwrap_or(&""), e.username, e.value))
        .collect::<Vec<_>>().join("\n");

    let level_ranking = top_level.iter().enumerate()
        .map(|(i, e)| format!("{} **{}** — Niveau {}", medals.get(i).unwrap_or(&""), e.username, e.value))
        .collect::<Vec<_>>().join("\n");

    let thieves_ranking = top_thieves.iter().enumerate()
        .map(|(i, e)| format!("{} **{}** — {} coins voles", medals.get(i).unwrap_or(&""), e.username, e.value))
        .collect::<Vec<_>>().join("\n");

    let (title, description) = match &season {
        Some(s) => {
            // started_at est au format ISO ('2026-04-10 12:34:56+00'), on garde la date
            let started_date = s.started_at.split(&[' ', 'T'][..]).next().unwrap_or(&s.started_at);
            let title = format!("\u{1f3c6} Saison {}", s.season_number);
            let description = format!(
                "Saison demarree le **{}**\n\u{23f3} Temps restant : **{} jours**\n\nLa saison dure 90 jours. Le joueur le plus riche a la fin sera couronne **Champion** !",
                started_date, s.days_remaining
            );
            (title, description)
        }
        None => (
            "\u{1f3c6} Saison en cours".to_string(),
            "La saison se termine tous les 3 mois. Le joueur le plus riche sera couronne **Champion** !".to_string(),
        ),
    };

    let embed = CreateEmbed::new()
        .title(title)
        .description(description)
        .field(
            "\u{1f4b0} Plus riches",
            if coins_ranking.is_empty() { "Aucun joueur".to_string() } else { coins_ranking },
            false,
        )
        .field(
            "\u{2b50} Plus haut niveau",
            if level_ranking.is_empty() { "Aucun joueur".to_string() } else { level_ranking },
            false,
        )
        .field(
            "\u{1f5e1}\u{fe0f} Plus gros voleurs",
            if thieves_ranking.is_empty() { "Aucun joueur".to_string() } else { thieves_ranking },
            false,
        )
        .color(0xF1C40F)
        .footer(CreateEmbedFooter::new(format!(
            "{} — Reset saisonnier tous les 3 mois",
            sentinel_shared::branding::COUDE_TAGLINE_SHORT,
        )))
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

