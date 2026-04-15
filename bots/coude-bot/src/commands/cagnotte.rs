//! Commande /cagnotte — affiche l'etat de la caisse communautaire (Phase 9).
//!
//! La caisse collecte tous les coins "perdus" par l'economie (shop, assurance,
//! taxes, etc.) et est redistribuee chaque semaine aleatoirement aux joueurs
//! actifs. Cette commande permet aux joueurs de suivre combien est accumule
//! et la date de la derniere redistribution.

use serenity::all::{
    CommandInteraction, Context, CreateCommand, CreateEmbed, CreateEmbedFooter,
    CreateInteractionResponse, CreateInteractionResponseMessage,
};

use crate::handler::load_guild_config;
use crate::GameApiKey;

pub fn register() -> CreateCommand {
    CreateCommand::new("cagnotte")
        .description("Affiche l'argent accumule dans la caisse communautaire")
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
    if !config.enabled() {
        reply_ephemeral(ctx, command, "Le jeu Coup de Coude est desactive sur ce serveur.").await;
        return;
    }

    let data = ctx.data.read().await;
    let api = match data.get::<GameApiKey>() {
        Some(a) => a,
        None => return,
    };

    let cashbox = match api.get_cashbox(&guild_id).await {
        Ok(c) => c,
        Err(e) => {
            reply_ephemeral(ctx, command, &format!("Erreur API : {e}")).await;
            return;
        }
    };

    let last_redist_field = match &cashbox.last_redistribution_at {
        Some(ts) => {
            let date = ts.split(&[' ', 'T'][..]).next().unwrap_or(ts);
            format!("\u{1f4c5} **{}**", date)
        }
        None => "_Jamais encore redistribuee_".to_string(),
    };

    let description = if cashbox.balance > 0 {
        format!(
            "\u{1fa99} **{}** coins en attente de redistribution !\n\n\
             La caisse est redistribuee aleatoirement chaque semaine aux joueurs actifs. \
             Plus tu joues, plus tu as de chances d'en profiter.",
            cashbox.balance
        )
    } else {
        "_La caisse est vide pour le moment._\n\nLes coins depenses au shop, en assurance, en taxes ou en penalites viennent s'y accumuler.".to_string()
    };

    let embed = CreateEmbed::new()
        .title("\u{1f3b0} Cagnotte communautaire")
        .description(description)
        .field(
            "\u{1fa99} Solde actuel",
            format!("**{}** coins", cashbox.balance),
            true,
        )
        .field(
            "\u{1f4e5} Total collecte",
            format!("{} coins", cashbox.total_collected),
            true,
        )
        .field(
            "\u{1f4e4} Total redistribue",
            format!("{} coins", cashbox.total_redistributed),
            true,
        )
        .field("\u{23f0} Derniere redistribution", last_redist_field, false)
        .color(0xF39C12)
        .footer(CreateEmbedFooter::new(
            "Coup de Coude | Sentinel — Redistribution hebdomadaire aux joueurs actifs",
        ))
        .timestamp(serenity::model::Timestamp::now());

    if let Err(e) = command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new().embed(embed),
            ),
        )
        .await
    {
        tracing::warn!(error = %e, "Echec response Discord");
    }
}

async fn reply_ephemeral(ctx: &Context, command: &CommandInteraction, content: &str) {
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
        tracing::warn!(error = %e, "Echec response Discord");
    }
}
