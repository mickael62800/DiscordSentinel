use serenity::all::{
    CommandInteraction, Context, CreateCommand, CreateEmbed, CreateEmbedFooter,
    CreateInteractionResponse, CreateInteractionResponseMessage,
};

use crate::handler::load_guild_config;
use crate::GameApiKey;

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
    if !crate::channel_check::check_channel(ctx, command, config.channel_profil()).await {
        return;
    }

    let data = ctx.data.read().await;
    let api = match data.get::<GameApiKey>() {
        Some(a) => a,
        None => return,
    };

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

    // TODO: Recuperer la saison en cours depuis l'API (numero, date de debut, temps restant)
    // Pour l'instant on affiche un placeholder

    let embed = CreateEmbed::new()
        .title("\u{1f3c6} Saison en cours")
        .description("La saison se termine tous les 3 mois. Le joueur le plus riche sera couronne **Champion** !")
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
        .footer(CreateEmbedFooter::new("Coup de Coude | Sentinel — Reset saisonnier tous les 3 mois"))
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

async fn reply_ephemeral(ctx: &Context, command: &CommandInteraction, content: &str) {
    command.create_response(&ctx.http, CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new().content(content).ephemeral(true),
    )).await.ok();
}
