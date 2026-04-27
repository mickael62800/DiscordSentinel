//! Commande `/prank` — pranks communautaires (cf. COUPE_AMELIORATIONS 5.4).
//!
//! Outils de troll pur, zero gameplay derriere, juste de l ambiance.
//! Les coins payes sont des gold sinks (debit du wallet, pas de
//! redistribution).
//!
//! Types implementes : braquage (100c), scoop (200c), appel (50c).
//! `costume` (300c) pas implemente : trop intrusif (hooker chaque
//! message d un user).

use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context, CreateCommand,
    CreateCommandOption, CreateEmbed, CreateEmbedFooter, CreateMessage,
};

use sentinel_shared::discord_helpers::{reply_ephemeral, require_guild_id, reply_api_err};

use crate::modules::coude::load_guild_config;
use crate::modules::coude::GameApiKey;

// Couts migres dans `CoudeConfig` (Phase 1 leftovers audit) :
//   prank_braquage_cost / prank_scoop_cost / prank_appel_cost.
// Defaults preserves : 100c / 200c / 50c.
//
// Templates (SCOOP / FAUX_APPEL) migres dans `coude_flavor_templates`
// (Phase 3 #9). Le bot consomme via `api.random_flavor`. Pas de fallback
// local — si l'API est indispo on affiche un message d'erreur.
//
// Faux montant /braquage migre dans l'endpoint
// `POST /prank/braquage/roll` (Phase 3 finalisation).

pub fn register() -> CreateCommand {
    CreateCommand::new("prank")
        .description("Outils de troll communautaires (cf. roadmap 5.4)")
        .add_option(
            CreateCommandOption::new(CommandOptionType::String, "type", "Type de prank")
                .required(true)
                .add_string_choice("Fausse alerte braquage (100c)", "braquage")
                .add_string_choice("Faux scoop sur un pote (200c)", "scoop")
                .add_string_choice("Faux appel en DM (50c)", "appel"),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::User,
                "cible",
                "Cible (obligatoire pour scoop / appel)",
            )
            .required(false),
        )
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    let Some(guild_id) = require_guild_id(ctx, command).await else { return; };

    let config = load_guild_config(ctx, &guild_id).await;
    if !crate::modules::coude::channel_check::check_channel(ctx, command, config.channel_activites()).await {
        return;
    }

    let prank_type = command
        .data
        .options
        .iter()
        .find(|o| o.name == "type")
        .and_then(|o| match &o.value {
            CommandDataOptionValue::String(s) => Some(s.clone()),
            _ => None,
        })
        .unwrap_or_default();

    let target_id_opt = command
        .data
        .options
        .iter()
        .find(|o| o.name == "cible")
        .and_then(|o| match &o.value {
            CommandDataOptionValue::User(id) => Some(*id),
            _ => None,
        });

    let source_id = command.user.id.to_string();
    let cost = match prank_type.as_str() {
        "braquage" => config.prank_braquage_cost(),
        "scoop" => config.prank_scoop_cost(),
        "appel" => config.prank_appel_cost(),
        _ => {
            reply_ephemeral(ctx, command, "Type de prank inconnu.").await;
            return;
        }
    };

    // Validations dependant du type.
    let target_user = if matches!(prank_type.as_str(), "scoop" | "appel") {
        let Some(tid) = target_id_opt else {
            reply_ephemeral(ctx, command, "Ce prank necessite une cible.").await;
            return;
        };
        match tid.to_user(&ctx.http).await {
            Ok(u) if u.bot => {
                reply_ephemeral(ctx, command, "Pas de prank contre un bot.").await;
                return;
            }
            Ok(u) => Some(u),
            Err(_) => {
                reply_ephemeral(ctx, command, "Utilisateur introuvable.").await;
                return;
            }
        }
    } else {
        None
    };

    let data = ctx.data.read().await;
    let api = data.get::<GameApiKey>().unwrap();

    let player = match api
        .get_or_create_player(&guild_id, &source_id, &command.user.name)
        .await
    {
        Ok(p) => p,
        Err(e) => {
            reply_api_err(ctx, command, e).await;
            return;
        }
    };
    if player.coins < cost {
        reply_ephemeral(
            ctx,
            command,
            &format!("Pas assez de coins ! Il te faut {cost}c."),
        )
        .await;
        return;
    }

    if let Err(e) = api.update_player_coins(&guild_id, &source_id, -cost).await {
        reply_api_err(ctx, command, e).await;
        return;
    }

    match prank_type.as_str() {
        "braquage" => execute_braquage(ctx, command, &config).await,
        "scoop" => execute_scoop(ctx, command, &config, target_user.as_ref().unwrap()).await,
        "appel" => execute_appel(ctx, command, target_user.as_ref().unwrap()).await,
        _ => unreachable!(),
    }
}

async fn execute_braquage(
    ctx: &Context,
    command: &CommandInteraction,
    config: &crate::modules::coude::guild_config::CoudeConfig,
) {
    let guild_id = command
        .guild_id
        .map(|g| g.to_string())
        .unwrap_or_default();
    // Faux montant tire cote API (catalogue editable, decision auditable).
    let fake_amount = {
        let data = ctx.data.read().await;
        let api = data.get::<GameApiKey>().unwrap();
        match api.roll_prank_braquage_amount(&guild_id).await {
            Ok(r) => r.amount,
            Err(_) => {
                reply_ephemeral(
                    ctx,
                    command,
                    "API indispo, veuillez reessayer plus tard.",
                )
                .await;
                return;
            }
        }
    };

    let embed = CreateEmbed::new()
        .title("\u{1f6a8} BRAQUAGE EN COURS !")
        .description(format!(
            "**ALERTE !** Un braquage est en cours !\n\
             La cagnotte serveur affiche **{} coins** !\n\n\
             Tout le monde sur le pont !!! \u{1f4b0}",
            fake_amount
        ))
        .color(0xE74C3C)
        .footer(CreateEmbedFooter::new(format!(
            "(prank pose par {})",
            command.user.name
        )))
        .timestamp(serenity::model::Timestamp::now());

    crate::modules::coude::channel_check::post_activity(
        ctx,
        command,
        config.channel_activites(),
        embed,
    )
    .await;
}

async fn execute_scoop(
    ctx: &Context,
    command: &CommandInteraction,
    config: &crate::modules::coude::guild_config::CoudeConfig,
    target: &serenity::model::user::User,
) {
    // Tirage cote API (catalogue `prank_scoop`).
    let tmpl: String = {
        let data = ctx.data.read().await;
        let api = data.get::<GameApiKey>().unwrap();
        match api.random_flavor("prank_scoop", "fr").await {
            Ok(Some(s)) => s,
            Ok(None) | Err(_) => {
                reply_ephemeral(
                    ctx,
                    command,
                    "API indispo, veuillez reessayer plus tard.",
                )
                .await;
                return;
            }
        }
    };
    let body = tmpl.replace("{cible}", &format!("<@{}>", target.id));

    let embed = CreateEmbed::new()
        .title("\u{1f4f0} SCOOP")
        .description(body)
        .color(0xF1C40F)
        .footer(CreateEmbedFooter::new(format!(
            "(rumeur infondee posee par {})",
            command.user.name
        )))
        .timestamp(serenity::model::Timestamp::now());

    crate::modules::coude::channel_check::post_activity(
        ctx,
        command,
        config.channel_activites(),
        embed,
    )
    .await;
}

async fn execute_appel(
    ctx: &Context,
    command: &CommandInteraction,
    target: &serenity::model::user::User,
) {
    // Tirage cote API (catalogue `prank_appel`).
    let tmpl: String = {
        let data = ctx.data.read().await;
        let api = data.get::<GameApiKey>().unwrap();
        match api.random_flavor("prank_appel", "fr").await {
            Ok(Some(s)) => s,
            Ok(None) | Err(_) => {
                reply_ephemeral(
                    ctx,
                    command,
                    "API indispo, veuillez reessayer plus tard.",
                )
                .await;
                return;
            }
        }
    };

    let embed = CreateEmbed::new()
        .title("\u{1f4de} Notification automatique")
        .description(tmpl)
        .color(0x57F287)
        .footer(CreateEmbedFooter::new(
            "Bot officiel — ce message est genere automatiquement",
        ))
        .timestamp(serenity::model::Timestamp::now());

    let dm_result = target.id.create_dm_channel(&ctx.http).await;
    let mut delivered = false;
    if let Ok(channel) = dm_result {
        if channel
            .send_message(&ctx.http, CreateMessage::new().embed(embed))
            .await
            .is_ok()
        {
            delivered = true;
        }
    }

    let confirmation = if delivered {
        format!(
            "DM envoye a <@{}> ! Attends de voir s il essaie le `/claim` qui n existe pas... \u{1f608}",
            target.id
        )
    } else {
        format!(
            "Impossible d envoyer un DM a <@{}> (DM ferme ?). Tes coins sont quand meme partis, desole.",
            target.id
        )
    };
    reply_ephemeral(ctx, command, &confirmation).await;
}
