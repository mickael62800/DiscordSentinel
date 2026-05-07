//! Commande `/travaux` — taches communautaires en prison
//! (cf. COUPE_AMELIORATIONS 4.3).
//!
//! Phase 2 #2 audit : RNG (selection tache + outcome + montant + flavor)
//! migre cote API. Le bot envoie la commande et affiche le verdict.

use serenity::all::{
    CommandInteraction, Context, CreateCommand, CreateEmbed, CreateEmbedFooter,
};

use crate::shared::discord_helpers::{reply_ephemeral, require_guild_id};

use crate::modules::coude::api_client::PlayTravauxResp;
use crate::modules::coude::load_guild_config;
use crate::modules::coude::GameApiKey;

pub fn register() -> CreateCommand {
    CreateCommand::new("travaux")
        .description("Effectue une tache de prison (disponible uniquement en cellule)")
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    let Some(guild_id) = require_guild_id(ctx, command).await else { return; };
    let config = load_guild_config(ctx, &guild_id).await;
    if !crate::modules::coude::channel_check::check_channel(ctx, command, config.channel_activites()).await {
        return;
    }

    let user_id = command.user.id.to_string();
    let data = ctx.data.read().await;
    let api = data.get::<GameApiKey>().unwrap();

    // Appel API atomique : prison check + cooldown + RNG + credit + XP +
    // pose cooldown. Erreurs (Forbidden / RateLimited / Internal) sont
    // affichees telles quelles.
    let resp: PlayTravauxResp = match api.play_travaux(&guild_id, &user_id, &command.user.name).await {
        Ok(r) => r,
        Err(e) => {
            reply_ephemeral(ctx, command, &e).await;
            return;
        }
    };

    let title = if resp.success {
        format!("\u{2705} {} — Reussi !", resp.task_label)
    } else {
        format!("\u{274c} {} — Echec.", resp.task_label)
    };
    let body = if resp.success {
        format!(
            "_{}_\n\n{}\n\n\u{1f4b0} **+{}c** + **{} XP**.\nProchaine tache dans 2h.",
            resp.task_description, resp.flavor, resp.coins_gain, resp.xp_gain
        )
    } else {
        format!(
            "_{}_\n\n{}\n\nProchaine tache dans 2h.",
            resp.task_description, resp.flavor
        )
    };
    let embed = CreateEmbed::new()
        .title(title)
        .description(body)
        .color(if resp.success { 0x2ECC71 } else { 0x95A5A6 })
        .footer(CreateEmbedFooter::new(
            crate::shared::branding::COUDE_TAGLINE_SHORT,
        ))
        .timestamp(serenity::model::Timestamp::now());

    crate::modules::coude::channel_check::post_activity(
        ctx,
        command,
        config.channel_activites(),
        embed,
    )
    .await;
}
