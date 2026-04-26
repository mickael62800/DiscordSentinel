//! Slash command `/coude-amical` — duel d'entrainement sans mise.
//!
//! Cf. COUPE_AMELIORATIONS 4.5. Aucun coin n'est jamais transfere.
//! Le moteur de combat tourne normalement (memes classes, items, regles)
//! mais cote economie : zero. XP gagne (+20 winner / +5 loser) et
//! statistiques separees (`friendly_wins` / `friendly_losses`).

use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context, CreateCommand,
    CreateCommandOption, CreateEmbed, CreateEmbedFooter,
};

use sentinel_shared::discord_helpers::reply_ephemeral;

use crate::modules::coude::load_guild_config;
use crate::modules::coude::GameApiKey;

pub fn register() -> CreateCommand {
    CreateCommand::new("coude-amical")
        .description("Duel d'entrainement sans mise — pour tester sans risque")
        .add_option(
            CreateCommandOption::new(CommandOptionType::User, "cible", "Le joueur a defier")
                .required(true),
        )
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
    if !crate::modules::coude::channel_check::check_channel(ctx, command, config.channel_combats()).await {
        return;
    }
    if !config.enabled() {
        reply_ephemeral(ctx, command, "Le jeu Coup de Coude est desactive sur ce serveur.").await;
        return;
    }

    let target_id = command
        .data
        .options
        .iter()
        .find(|o| o.name == "cible")
        .and_then(|o| match &o.value {
            CommandDataOptionValue::User(id) => Some(*id),
            _ => None,
        });
    let target_id = match target_id {
        Some(id) => id,
        None => {
            reply_ephemeral(ctx, command, "Cible invalide.").await;
            return;
        }
    };

    if target_id == command.user.id {
        reply_ephemeral(ctx, command, "Tu ne peux pas te defier toi-meme !").await;
        return;
    }

    let target = match target_id.to_user(&ctx.http).await {
        Ok(u) => u,
        Err(_) => {
            reply_ephemeral(ctx, command, "Utilisateur introuvable.").await;
            return;
        }
    };

    if target.bot {
        reply_ephemeral(ctx, command, "Tu ne peux pas defier un bot !").await;
        return;
    }

    if !crate::modules::coude::interaction_helper::defer_response(ctx, command).await {
        return;
    }

    let data = ctx.data.read().await;
    let api = data.get::<GameApiKey>().unwrap();

    let resp = match api
        .resolve_friendly_duel(
            &guild_id,
            &command.user.id.to_string(),
            &command.user.name,
            &target.id.to_string(),
            &target.name,
        )
        .await
    {
        Ok(r) => r,
        Err(e) => {
            crate::modules::coude::interaction_helper::followup_text(
                ctx,
                command,
                &format!("Erreur API : {e}"),
            )
            .await;
            return;
        }
    };

    let title = if resp.draw {
        "\u{1f91d} Duel amical — Egalite".to_string()
    } else if resp.winner_id.as_deref() == Some(&command.user.id.to_string()) {
        format!("\u{1f3c6} Duel amical — Victoire de <@{}>", command.user.id)
    } else if resp.winner_id.as_deref() == Some(&target.id.to_string()) {
        format!("\u{1f3c6} Duel amical — Victoire de <@{}>", target.id)
    } else {
        "\u{1f91d} Duel amical".to_string()
    };

    let xp_line = if resp.draw {
        format!(
            "\u{2b06}\u{fe0f} +{} XP pour les deux (egalite)",
            resp.loser_xp
        )
    } else {
        format!(
            "\u{2b06}\u{fe0f} +{} XP pour le gagnant, +{} XP pour le perdant",
            resp.winner_xp, resp.loser_xp
        )
    };

    let embed = CreateEmbed::new()
        .title(title)
        .description(format!(
            "**Combat d'entrainement** — aucun coin transfere.\n\n\
             Duree : {} rounds\n\
             <@{}> : {}/{} HP\n\
             <@{}> : {}/{} HP\n\n\
             {}",
            resp.total_rounds,
            command.user.id,
            resp.attacker_hp_final,
            resp.attacker_hp_max,
            target.id,
            resp.defender_hp_final,
            resp.defender_hp_max,
            xp_line,
        ))
        .color(0x3498DB)
        .footer(CreateEmbedFooter::new(format!(
            "{} — duel amical (stats separees)",
            sentinel_shared::branding::COUDE_TAGLINE_SHORT,
        )))
        .timestamp(serenity::model::Timestamp::now());

    crate::modules::coude::interaction_helper::followup_embed(ctx, command, embed).await;
}
