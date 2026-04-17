//! Slash command /security (status, history).

use serenity::all::{
    CommandInteraction, CommandOptionType, Context, CreateCommand,
    CreateCommandOption, CommandDataOptionValue,
};

use sentinel_shared::discord_helpers::reply_ephemeral_embed;
use sentinel_shared::embeds::info_embed;

use super::{
    LockdownKey, QuarantineKey, RaidDetectorKey, RecentJoinsKey, SecurityApiKey,
};

pub fn register() -> CreateCommand {
    CreateCommand::new("security")
        .description("Commandes du security bot")
        .default_member_permissions(serenity::all::Permissions::MANAGE_GUILD)
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "status",
                "Affiche l'etat actuel de la securite",
            ),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "history",
                "Affiche les derniers evenements de securite",
            )
            .add_sub_option(
                CreateCommandOption::new(
                    CommandOptionType::Integer,
                    "limit",
                    "Nombre d'evenements a afficher (defaut: 5)",
                )
                .min_int_value(1)
                .max_int_value(25)
                .required(false),
            ),
        )
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    let sub = command
        .data
        .options
        .first()
        .map(|o| o.name.as_str())
        .unwrap_or("");

    match sub {
        "status" => handle_status(ctx, command).await,
        "history" => handle_history(ctx, command).await,
        _ => {}
    }
}

async fn handle_status(ctx: &Context, command: &CommandInteraction) {
    let guild_id = match command.guild_id {
        Some(id) => id,
        None => return,
    };

    let data = ctx.data.read().await;

    let recent_joins = data
        .get::<RaidDetectorKey>()
        .map(|r| r.recent_joins(guild_id))
        .unwrap_or(0);

    let lockdown_active = data
        .get::<LockdownKey>()
        .map(|l| l.is_active(guild_id))
        .unwrap_or(false);

    let quarantined_count = data
        .get::<QuarantineKey>()
        .map(|q| q.quarantined_count())
        .unwrap_or(0);

    let recent_joins_tracker = data
        .get::<RecentJoinsKey>()
        .map(|r| r.recent(guild_id).len())
        .unwrap_or(0);

    let embed = info_embed("Security — Statut")
        .field("Joins recents (raid detector)", recent_joins.to_string(), true)
        .field(
            "Lockdown",
            if lockdown_active { "Actif" } else { "Inactif" },
            true,
        )
        .field("Utilisateurs en quarantaine", quarantined_count.to_string(), true)
        .field("Joins recents (tracker)", recent_joins_tracker.to_string(), true);

    reply_ephemeral_embed(ctx, command, embed).await;
}

async fn handle_history(ctx: &Context, command: &CommandInteraction) {
    let guild_id = match command.guild_id {
        Some(id) => id,
        None => return,
    };

    let limit = command
        .data
        .options
        .first()
        .and_then(|sub| {
            if let CommandDataOptionValue::SubCommand(opts) = &sub.value {
                opts.iter()
                    .find(|o| o.name == "limit")
                    .and_then(|o| o.value.as_i64())
            } else {
                None
            }
        })
        .unwrap_or(5) as u32;

    let data = ctx.data.read().await;
    let sec_api = match data.get::<SecurityApiKey>() {
        Some(a) => a,
        None => return,
    };

    match sec_api.list_events(&guild_id.to_string(), limit).await {
        Ok(events) => {
            let description = if events.is_empty() {
                "Aucun evenement recent.".to_string()
            } else {
                events
                    .iter()
                    .enumerate()
                    .map(|(i, e)| {
                        let event_type = e
                            .get("event_type")
                            .and_then(|v| v.as_str())
                            .unwrap_or("inconnu");
                        let severity = e
                            .get("severity")
                            .and_then(|v| v.as_str())
                            .unwrap_or("info");
                        let description = e
                            .get("description")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        let desc_preview = if description.chars().count() > 80 {
                            let truncated: String = description.chars().take(80).collect();
                            format!("{}...", truncated)
                        } else {
                            description.to_string()
                        };
                        format!(
                            "{}. [{}] **{}** — {}",
                            i + 1,
                            severity,
                            event_type,
                            desc_preview
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            };

            let embed = info_embed("Security — Historique").description(description);
            reply_ephemeral_embed(ctx, command, embed).await;
        }
        Err(e) => {
            let embed = info_embed("Security — Historique")
                .description(format!("Erreur lors de la recuperation : {}", e));
            reply_ephemeral_embed(ctx, command, embed).await;
        }
    }
}
