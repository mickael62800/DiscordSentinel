//! Slash command `/progression-resync` — re-synchronise les pseudos
//! `[NN]Pseudo` apres changement de niveau ou en backfill suite a un
//! deploiement.
//!
//! Sous-commandes :
//! - `/progression-resync user @cible`  — rename 1 membre
//! - `/progression-resync me`           — rename soi-meme
//! - `/progression-resync all`          — rename les top N du leaderboard
//!   global (admin only, MANAGE_GUILD).

use std::time::Duration;

use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context, CreateCommand,
    CreateCommandOption, CreateEmbed, CreateInteractionResponseFollowup, GuildId, Permissions,
    UserId,
};
use tokio::time::sleep;
use tracing::warn;

use crate::shared::discord_helpers::reply_ephemeral;

use super::nickname::{apply_level_prefix, ResyncOutcome};
use super::StatsApiKey;

/// Throttle entre 2 syncs en mode `all` : evite de cogner le rate limit
/// Discord.
const RESYNC_ALL_INTERVAL_MS: u64 = 250;
/// Plafond de users traites par `all`.
const RESYNC_ALL_MAX_USERS: u32 = 200;

pub fn register() -> CreateCommand {
    CreateCommand::new("progression-resync")
        .description("Re-synchronise les pseudos [NN]Pseudo selon le niveau global")
        .default_member_permissions(Permissions::MANAGE_GUILD)
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "user",
                "Re-applique le prefixe sur un membre precis",
            )
            .add_sub_option(
                CreateCommandOption::new(
                    CommandOptionType::User,
                    "target",
                    "Membre a re-synchroniser",
                )
                .required(true),
            ),
        )
        .add_option(CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "me",
            "Re-applique le prefixe sur ton propre pseudo",
        ))
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "all",
                "Re-applique le prefixe sur les top N joueurs (admin only)",
            )
            .add_sub_option(
                CreateCommandOption::new(
                    CommandOptionType::Integer,
                    "limit",
                    "Nombre max de joueurs (defaut 50, max 200)",
                )
                .min_int_value(1)
                .max_int_value(RESYNC_ALL_MAX_USERS as u64),
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

    let sub = match command.data.options.first() {
        Some(s) => s,
        None => {
            reply_ephemeral(ctx, command, "Sous-commande manquante.").await;
            return;
        }
    };

    match sub.name.as_str() {
        "user" => handle_single(ctx, command, guild_id, sub_user_target(sub)).await,
        "me" => handle_single(ctx, command, guild_id, Some(command.user.id)).await,
        "all" => handle_all(ctx, command, guild_id, sub_limit(sub)).await,
        _ => reply_ephemeral(ctx, command, "Sous-commande inconnue.").await,
    }
}

fn sub_user_target(sub: &serenity::all::CommandDataOption) -> Option<UserId> {
    let CommandDataOptionValue::SubCommand(opts) = &sub.value else {
        return None;
    };
    opts.iter()
        .find(|o| o.name == "target")
        .and_then(|o| match &o.value {
            CommandDataOptionValue::User(id) => Some(*id),
            _ => None,
        })
}

fn sub_limit(sub: &serenity::all::CommandDataOption) -> u32 {
    let CommandDataOptionValue::SubCommand(opts) = &sub.value else {
        return 50;
    };
    opts.iter()
        .find(|o| o.name == "limit")
        .and_then(|o| match &o.value {
            CommandDataOptionValue::Integer(v) => Some(*v as u32),
            _ => None,
        })
        .unwrap_or(50)
        .min(RESYNC_ALL_MAX_USERS)
}

async fn handle_single(
    ctx: &Context,
    command: &CommandInteraction,
    guild_id: GuildId,
    target: Option<UserId>,
) {
    let target = match target {
        Some(t) => t,
        None => {
            reply_ephemeral(ctx, command, "Cible invalide.").await;
            return;
        }
    };

    if !defer_ephemeral(ctx, command).await {
        return;
    }

    let level = match fetch_level(ctx, &guild_id.to_string(), &target.to_string()).await {
        Ok(lvl) => lvl,
        Err(msg) => {
            followup(ctx, command, &msg).await;
            return;
        }
    };

    let outcome = apply_level_prefix(ctx, guild_id, target, level).await;
    let embed = build_single_embed(target, level, &outcome);
    if let Err(e) = command
        .create_followup(
            &ctx.http,
            CreateInteractionResponseFollowup::new()
                .embed(embed)
                .ephemeral(true),
        )
        .await
    {
        warn!(error = %e, "Echec followup resync user");
    }
}

async fn handle_all(ctx: &Context, command: &CommandInteraction, guild_id: GuildId, limit: u32) {
    if !defer_ephemeral(ctx, command).await {
        return;
    }

    let guild_str = guild_id.to_string();

    let leaderboard = {
        let data = ctx.data.read().await;
        let api = match data.get::<StatsApiKey>() {
            Some(a) => a,
            None => {
                drop(data);
                followup(ctx, command, "API indisponible.").await;
                return;
            }
        };
        match api.get_level_leaderboard(&guild_str, limit, None).await {
            Ok(list) => list,
            Err(e) => {
                drop(data);
                followup(ctx, command, &e).await;
                return;
            }
        }
    };

    if leaderboard.is_empty() {
        followup(ctx, command, "Aucun joueur trouve sur ce serveur.").await;
        return;
    }

    let total = leaderboard.len();
    let mut renamed = 0usize;
    let mut already_ok = 0usize;
    let mut skipped = 0usize;
    let mut errors = 0usize;

    for entry in leaderboard.into_iter() {
        let user_id = match entry.user_id.parse::<u64>() {
            Ok(v) => UserId::new(v),
            Err(_) => {
                skipped += 1;
                continue;
            }
        };
        match apply_level_prefix(ctx, guild_id, user_id, entry.level).await {
            ResyncOutcome::Renamed => renamed += 1,
            ResyncOutcome::AlreadyOk => already_ok += 1,
            ResyncOutcome::Skipped => skipped += 1,
            ResyncOutcome::Error(_) => errors += 1,
        }
        sleep(Duration::from_millis(RESYNC_ALL_INTERVAL_MS)).await;
    }

    let embed = CreateEmbed::new()
        .title("\u{1f504} Resync des pseudos — termine")
        .description(format!(
            "**{total}** joueurs traites (top XP global).\n\n\
             - \u{270f}\u{fe0f} Renommes : **{renamed}**\n\
             - \u{2705} Deja a jour : **{already_ok}**\n\
             - \u{23ed}\u{fe0f} Skipped (owner, parti, etc.) : **{skipped}**\n\
             - \u{26a0}\u{fe0f} Erreurs Discord : **{errors}**"
        ))
        .color(if errors > 0 { 0xF1C40F } else { 0x57F287 });

    if let Err(e) = command
        .create_followup(
            &ctx.http,
            CreateInteractionResponseFollowup::new()
                .embed(embed)
                .ephemeral(true),
        )
        .await
    {
        warn!(error = %e, "Echec followup resync all");
    }
}

/// Recupere le niveau global du joueur via l'API.
async fn fetch_level(ctx: &Context, guild_id: &str, user_id: &str) -> Result<i32, String> {
    let data = ctx.data.read().await;
    let api = data
        .get::<StatsApiKey>()
        .ok_or_else(|| "API indisponible".to_string())?;
    match api.get_user_level(guild_id, user_id).await {
        Ok(Some(u)) => Ok(u.level),
        Ok(None) => Err("Ce membre n'a pas encore d'XP.".to_string()),
        Err(e) => Err(e),
    }
}

fn build_single_embed(user_id: UserId, level: i32, outcome: &ResyncOutcome) -> CreateEmbed {
    let (status, color) = match outcome {
        ResyncOutcome::Renamed => ("\u{270f}\u{fe0f} Pseudo mis a jour", 0x3498DB),
        ResyncOutcome::AlreadyOk => ("\u{2705} Deja a jour", 0x57F287),
        ResyncOutcome::Skipped => (
            "\u{23ed}\u{fe0f} Skipped (owner / member introuvable)",
            0x95A5A6,
        ),
        ResyncOutcome::Error(msg) => {
            return CreateEmbed::new()
                .title("\u{1f504} Resync pseudo")
                .description(format!(
                    "<@{user_id}> — niveau **{level}**\n\n\u{26a0}\u{fe0f} Erreur Discord : {msg}"
                ))
                .color(0xE74C3C);
        }
    };

    CreateEmbed::new()
        .title("\u{1f504} Resync pseudo")
        .description(format!("<@{user_id}> — niveau **{level}**\n\n{status}"))
        .color(color)
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
        .map_err(|e| warn!(error = %e, "defer resync echoue"))
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
        warn!(error = %e, "Echec followup resync");
    }
}
