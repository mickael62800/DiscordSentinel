//! Slash command `/progression-resync` — force la verification et
//! reapplication des roles de niveau (text / voice / days) pour un
//! membre ou pour tout le serveur.
//!
//! Utile quand :
//! - Un admin a ajoute / modifie un reward apres coup (cache 5 min ou
//!   role manque a des users existants).
//! - Le mode `xp_role_mode` (separate / max / total) a ete change.
//! - Une attribution Discord a foire (rate limit, perms manquantes au
//!   moment du level-up).
//!
//! Sub-commands :
//! - `/progression-resync user @cible`  — re-applique pour 1 user
//! - `/progression-resync me`           — re-applique pour soi-meme
//! - `/progression-resync all`          — re-applique pour les top N
//!   joueurs du leaderboard global (admin only, MANAGE_GUILD).

use std::time::Duration;

use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context, CreateCommand,
    CreateCommandOption, CreateEmbed, CreateInteractionResponseFollowup, GuildId, Permissions,
    UserId,
};
use tokio::time::sleep;
use tracing::warn;

use sentinel_shared::discord_helpers::reply_ephemeral;

use super::{sync_member_roles, RoleSyncReport, StatsApiKey};

/// Throttle entre 2 syncs en mode `all` : evite de cogner le rate limit
/// Discord (5 reqs/s par bot, 1 sync = jusqu a ~6 calls add/remove role).
const RESYNC_ALL_INTERVAL_MS: u64 = 250;
/// Plafond de users traites par `all` : protege contre les serveurs
/// massifs ou la commande tournerait en boucle pendant des minutes.
const RESYNC_ALL_MAX_USERS: u32 = 200;

pub fn register() -> CreateCommand {
    CreateCommand::new("progression-resync")
        .description("Force la verification des roles de niveau (texte/vocal/jours)")
        .default_member_permissions(Permissions::MANAGE_GUILD)
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "user",
                "Re-verifie un membre precis",
            )
            .add_sub_option(
                CreateCommandOption::new(
                    CommandOptionType::User,
                    "target",
                    "Membre a re-verifier",
                )
                .required(true),
            ),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "me",
                "Re-verifie tes propres roles",
            ),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "all",
                "Re-verifie les top N joueurs du serveur (long, admin only)",
            )
            .add_sub_option(
                CreateCommandOption::new(
                    CommandOptionType::Integer,
                    "limit",
                    "Nombre max de joueurs a traiter (defaut 50, max 200)",
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
    opts.iter().find(|o| o.name == "target").and_then(|o| match &o.value {
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

    let levels = match fetch_levels(ctx, &guild_id.to_string(), &target.to_string()).await {
        Ok(l) => l,
        Err(msg) => {
            followup(ctx, command, &msg).await;
            return;
        }
    };

    let report = sync_member_roles(
        ctx,
        guild_id,
        target,
        levels.0,
        levels.1,
        levels.2,
    )
    .await;

    let embed = build_single_embed(target, &report, levels);
    if let Err(e) = command
        .create_followup(
            &ctx.http,
            CreateInteractionResponseFollowup::new().embed(embed).ephemeral(true),
        )
        .await
    {
        warn!(error = %e, "Echec followup resync user");
    }
}

async fn handle_all(
    ctx: &Context,
    command: &CommandInteraction,
    guild_id: GuildId,
    limit: u32,
) {
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
        // get_level_leaderboard renvoie les N joueurs ayant le plus d'XP
        // global. C'est l'ensemble pertinent — un user a 0 XP n'a aucun
        // role de niveau a synchroniser de toute facon.
        match api.get_level_leaderboard(&guild_str, limit, None).await {
            Ok(list) => list,
            Err(e) => {
                drop(data);
                followup(ctx, command, &format!("Erreur API : {e}")).await;
                return;
            }
        }
    };

    if leaderboard.is_empty() {
        followup(ctx, command, "Aucun joueur trouve sur ce serveur.").await;
        return;
    }

    let total = leaderboard.len();
    let mut total_added = 0usize;
    let mut total_removed = 0usize;
    let mut total_errors = 0usize;
    let mut skipped = 0usize;
    let mut affected_users = 0usize;

    for entry in leaderboard.into_iter() {
        let user_id = match entry.user_id.parse::<u64>() {
            Ok(v) => UserId::new(v),
            Err(_) => {
                skipped += 1;
                continue;
            }
        };

        let report = sync_member_roles(
            ctx,
            guild_id,
            user_id,
            entry.level_text,
            entry.level_voice,
            entry.level,
        )
        .await;

        if report.skipped {
            skipped += 1;
        } else if !report.added_roles.is_empty() || !report.removed_roles.is_empty() {
            affected_users += 1;
        }
        total_added += report.added_roles.len();
        total_removed += report.removed_roles.len();
        total_errors += report.errors.len();

        sleep(Duration::from_millis(RESYNC_ALL_INTERVAL_MS)).await;
    }

    let embed = CreateEmbed::new()
        .title("\u{1f504} Resync de progression — termine")
        .description(format!(
            "**{}** joueurs traites (top XP global).\n\n\
             - \u{2795} Roles attribues : **{}**\n\
             - \u{2796} Roles retires : **{}**\n\
             - \u{1f465} Membres modifies : **{}**\n\
             - \u{23ed}\u{fe0f} Skipped (parti du serveur, API HS, etc.) : **{}**\n\
             - \u{26a0}\u{fe0f} Erreurs Discord (perms / rate limit) : **{}**",
            total, total_added, total_removed, affected_users, skipped, total_errors
        ))
        .color(if total_errors > 0 { 0xF1C40F } else { 0x57F287 });

    if let Err(e) = command
        .create_followup(
            &ctx.http,
            CreateInteractionResponseFollowup::new().embed(embed).ephemeral(true),
        )
        .await
    {
        warn!(error = %e, "Echec followup resync all");
    }
}

/// Recupere les 3 niveaux (text, voice, global) du joueur via l API.
async fn fetch_levels(
    ctx: &Context,
    guild_id: &str,
    user_id: &str,
) -> Result<(i32, i32, i32), String> {
    let data = ctx.data.read().await;
    let api = data.get::<StatsApiKey>().ok_or_else(|| "API indisponible".to_string())?;
    match api.get_user_level(guild_id, user_id).await {
        Ok(Some(u)) => Ok((u.level_text, u.level_voice, u.level)),
        Ok(None) => Err("Ce membre n'a pas encore d'XP.".to_string()),
        Err(e) => Err(format!("Erreur API : {e}")),
    }
}

fn build_single_embed(
    user_id: UserId,
    report: &RoleSyncReport,
    levels: (i32, i32, i32),
) -> CreateEmbed {
    let (lt, lv, lg) = levels;
    let added_line = if report.added_roles.is_empty() {
        "_aucun_".to_string()
    } else {
        report.added_roles.iter().map(|id| format!("<@&{id}>")).collect::<Vec<_>>().join(", ")
    };
    let removed_line = if report.removed_roles.is_empty() {
        "_aucun_".to_string()
    } else {
        report.removed_roles.iter().map(|id| format!("<@&{id}>")).collect::<Vec<_>>().join(", ")
    };
    let errors_line = if report.errors.is_empty() {
        "_aucune_".to_string()
    } else {
        report.errors.join("\n")
    };

    let color = if !report.errors.is_empty() {
        0xE74C3C
    } else if report.skipped {
        0x95A5A6
    } else if !report.added_roles.is_empty() || !report.removed_roles.is_empty() {
        0x3498DB
    } else {
        0x57F287
    };

    let status = if report.skipped {
        "\u{23ed}\u{fe0f} Skipped (member ou API indisponible)"
    } else if !report.added_roles.is_empty() || !report.removed_roles.is_empty() {
        "\u{2705} Roles re-synchronises"
    } else {
        "\u{2705} Deja a jour"
    };

    CreateEmbed::new()
        .title("\u{1f504} Resync de progression")
        .description(format!(
            "<@{}> — niv. texte **{}** | vocal **{}** | global **{}**\n\n{}",
            user_id, lt, lv, lg, status
        ))
        .field("\u{2795} Ajoutes", added_line, false)
        .field("\u{2796} Retires", removed_line, false)
        .field("\u{26a0}\u{fe0f} Erreurs", errors_line, false)
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
            CreateInteractionResponseFollowup::new().content(content).ephemeral(true),
        )
        .await
    {
        warn!(error = %e, "Echec followup resync");
    }
}
