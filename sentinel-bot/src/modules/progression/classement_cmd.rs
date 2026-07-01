//! Slash command `/classement forcer` — publie le classement mensuel
//! d'activite (texte / vocal / global) A LA DEMANDE.
//!
//! Le job auto ne publie que si `monthly_ranking_enabled` est actif ET qu'une
//! baseline COMPLETE (non `partial`) existe pour le mois precedent : un jour-1
//! manque ou une activation en cours de mois fige le mois en `partial`, jamais
//! publie. Cette commande FORCE la publication immediatement (bypass des
//! gates cote API) et poste dans le salon configure, sinon le salon courant.
//!
//! Gate admin : MANAGE_GUILD (revalide a chaque call, cf. `/automod`).

use serenity::all::{
    ChannelId, CommandDataOptionValue, CommandInteraction, CommandOptionType, Context,
    CreateCommand, CreateCommandOption, CreateEmbed, CreateInteractionResponseFollowup,
    CreateMessage, Permissions, Timestamp,
};
use tracing::warn;

use crate::shared::api_client::BaseApiClient;
use crate::shared::discord_helpers::reply_ephemeral;
use crate::shared::heartbeat::ApiClientKey;

use super::api_client::{ForceRankingResponse, RankingEntry};
use super::{StatsApiKey, MODULE_BOT_NAME};

/// Couleur or, identique a l'embed du job auto.
const GOLD: u32 = 0xF1C40F;

/// MANAGE_GUILD est un simple hint UI cote Discord : on revalide explicitement.
fn has_manage_guild(command: &CommandInteraction) -> bool {
    command
        .member
        .as_ref()
        .and_then(|m| m.permissions)
        .map(|p| p.contains(Permissions::MANAGE_GUILD) || p.contains(Permissions::ADMINISTRATOR))
        .unwrap_or(false)
}

pub fn register() -> CreateCommand {
    CreateCommand::new("classement")
        .description("Classement mensuel d'activite (texte / vocal / global)")
        .default_member_permissions(Permissions::MANAGE_GUILD)
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "forcer",
                "Publie le classement mensuel maintenant (admin only)",
            )
            .add_sub_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "mois",
                    "Mois a publier (defaut : mois en cours)",
                )
                .add_string_choice("Mois en cours", "actuel")
                .add_string_choice("Mois precedent", "precedent"),
            ),
        )
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    let guild_id = match command.guild_id {
        Some(id) => id,
        None => {
            reply_ephemeral(ctx, command, "Commande serveur uniquement.").await;
            return;
        }
    };

    if !has_manage_guild(command) {
        reply_ephemeral(
            ctx,
            command,
            "Permission MANAGE_GUILD requise pour /classement.",
        )
        .await;
        return;
    }

    let sub = match command.data.options.first() {
        Some(s) if s.name == "forcer" => s,
        _ => {
            reply_ephemeral(ctx, command, "Sous-commande inconnue.").await;
            return;
        }
    };

    let mois = sub_mois(sub);

    if !defer_ephemeral(ctx, command).await {
        return;
    }

    let guild_str = guild_id.to_string();

    // Recupere les donnees (API) + le salon configure (config guild).
    let (ranking, channel_cfg) = {
        let data = ctx.data.read().await;
        let api = match data.get::<StatsApiKey>() {
            Some(a) => a,
            None => {
                drop(data);
                followup(ctx, command, "API indisponible.").await;
                return;
            }
        };

        let ranking = match api.force_monthly_ranking(&guild_str, mois).await {
            Ok(r) => r,
            Err(e) => {
                drop(data);
                followup(ctx, command, &e).await;
                return;
            }
        };

        // Salon configure (peut etre vide -> fallback salon courant).
        let channel_cfg = if let Some(base) = data.get::<ApiClientKey>() {
            let gc = base
                .get_guild_config_for(&guild_str, MODULE_BOT_NAME)
                .await
                .unwrap_or_default();
            BaseApiClient::config_or(&gc, "monthly_ranking_channel_id", "")
        } else {
            String::new()
        };

        (ranking, channel_cfg)
    };

    let embed = build_ranking_embed(&ranking);

    // Salon cible : configure si valide, sinon le salon d'invocation.
    let target = channel_cfg
        .trim()
        .parse::<u64>()
        .ok()
        .map(ChannelId::new)
        .unwrap_or(command.channel_id);

    match target
        .send_message(&ctx.http, CreateMessage::new().embed(embed))
        .await
    {
        Ok(_) => {
            followup(
                ctx,
                command,
                &format!("Classement publie dans <#{target}>."),
            )
            .await;
        }
        Err(e) => {
            warn!(error = %e, channel = %target, "Echec publication classement force");
            followup(
                ctx,
                command,
                "Impossible de poster dans le salon cible (permissions ?).",
            )
            .await;
        }
    }
}

fn sub_mois(sub: &serenity::all::CommandDataOption) -> &str {
    let CommandDataOptionValue::SubCommand(opts) = &sub.value else {
        return "actuel";
    };
    opts.iter()
        .find(|o| o.name == "mois")
        .and_then(|o| match &o.value {
            CommandDataOptionValue::String(s) if s == "precedent" => Some("precedent"),
            CommandDataOptionValue::String(s) if s == "actuel" => Some("actuel"),
            _ => None,
        })
        .unwrap_or("actuel")
}

/// Construit un bloc de classement (memes visuels que le job auto).
fn build_block(entries: &[RankingEntry]) -> String {
    if entries.is_empty() {
        return "_Aucune activite ce mois-ci._".to_string();
    }
    entries
        .iter()
        .enumerate()
        .map(|(i, e)| format!("**{}.** <@{}> \u{2014} {} XP", i + 1, e.user_id, e.xp))
        .collect::<Vec<_>>()
        .join("\n")
}

fn build_ranking_embed(r: &ForceRankingResponse) -> CreateEmbed {
    let mut description =
        "Les membres les plus actifs du mois \u{2014} bravo \u{1f44f}".to_string();
    if let Some(note) = &r.note {
        description.push_str("\n\n");
        description.push_str(note);
    }

    CreateEmbed::new()
        .title(format!("\u{1f3c6} Classement de {}", r.period_label))
        .description(description)
        .color(GOLD)
        .field("\u{1f4dd} Top Texte", build_block(&r.text), false)
        .field("\u{1f399}\u{fe0f} Top Vocal", build_block(&r.voice), false)
        .field("\u{1f3c5} Top Global", build_block(&r.global), false)
        // Aligne sur l'embed du job auto (meme timestamp) pour eviter toute
        // derive cosmetique entre les deux rendus.
        .timestamp(Timestamp::now())
}

async fn defer_ephemeral(ctx: &Context, command: &CommandInteraction) -> bool {
    use serenity::all::{CreateInteractionResponse, CreateInteractionResponseMessage};
    command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Defer(
                CreateInteractionResponseMessage::new().ephemeral(true),
            ),
        )
        .await
        .map_err(|e| warn!(error = %e, "defer classement echoue"))
        .is_ok()
}

async fn followup(ctx: &Context, command: &CommandInteraction, content: &str) {
    if let Err(e) = command
        .create_followup(
            &ctx.http,
            CreateInteractionResponseFollowup::new()
                .content(content)
                .ephemeral(true),
        )
        .await
    {
        warn!(error = %e, "Echec followup classement");
    }
}
