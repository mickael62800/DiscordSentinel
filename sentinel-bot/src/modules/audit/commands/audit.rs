use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context,
    CreateCommand, CreateCommandOption,
};

use crate::shared::discord_helpers::reply_ephemeral_embed;
use crate::shared::embeds::info_embed;
use crate::shared::heartbeat::ApiClientKey;

use super::api_client::ApiClient;
use super::WeeklyTrackerKey;

pub fn register() -> CreateCommand {
    CreateCommand::new("audit")
        .description("Commandes du audit bot")
        .default_member_permissions(serenity::all::Permissions::MODERATE_MEMBERS)
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "search",
                "Rechercher dans les logs d'audit",
            )
            .add_sub_option(
                CreateCommandOption::new(
                    CommandOptionType::User,
                    "user",
                    "Utilisateur cible",
                )
                .required(true),
            )
            .add_sub_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "type",
                    "Type d'evenement (ex: message_delete, member_ban)",
                )
                .required(false),
            )
            .add_sub_option(
                CreateCommandOption::new(
                    CommandOptionType::Integer,
                    "limit",
                    "Nombre de resultats (defaut: 10)",
                )
                .min_int_value(1)
                .max_int_value(50)
                .required(false),
            ),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "stats",
                "Affiche les statistiques hebdomadaires",
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
        "search" => handle_search(ctx, command).await,
        "stats" => handle_stats(ctx, command).await,
        _ => {}
    }
}

async fn handle_search(ctx: &Context, command: &CommandInteraction) {
    let guild_id = match command.guild_id {
        Some(id) => id,
        None => return,
    };

    // Parse sub-command options
    let sub_opts = command
        .data
        .options
        .first()
        .and_then(|sub| {
            if let CommandDataOptionValue::SubCommand(opts) = &sub.value {
                Some(opts.clone())
            } else {
                None
            }
        })
        .unwrap_or_default();

    let target_id = sub_opts
        .iter()
        .find(|o| o.name == "user")
        .and_then(|o| {
            if let CommandDataOptionValue::User(id) = &o.value {
                Some(id.to_string())
            } else {
                None
            }
        });

    let event_type = sub_opts
        .iter()
        .find(|o| o.name == "type")
        .and_then(|o| o.value.as_str().map(|s| s.to_string()));

    let limit = sub_opts
        .iter()
        .find(|o| o.name == "limit")
        .and_then(|o| o.value.as_i64())
        .unwrap_or(10) as u32;

    let data = ctx.data.read().await;
    let base = match data.get::<ApiClientKey>() {
        Some(a) => a,
        None => return,
    };
    let api = ApiClient::new(base.clone());

    match api
        .search_audit_logs(
            &guild_id.to_string(),
            target_id.as_deref(),
            event_type.as_deref(),
            limit,
        )
        .await
    {
        Ok(logs) => {
            let description = if logs.is_empty() {
                "Aucun resultat.".to_string()
            } else {
                logs.iter()
                    .enumerate()
                    .map(|(i, e)| {
                        let etype = e
                            .get("event_type")
                            .and_then(|v| v.as_str())
                            .unwrap_or("inconnu");
                        let actor = e
                            .get("actor_name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("?");
                        let target = e
                            .get("target_name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("?");
                        format!("{}. **{}** par {} sur {}", i + 1, etype, actor, target)
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            };

            let embed = info_embed("Audit -- Recherche").description(description);
            reply_ephemeral_embed(ctx, command, embed).await;
        }
        Err(e) => {
            let embed = info_embed("Audit -- Recherche")
                .description(format!("Erreur : {}", e));
            reply_ephemeral_embed(ctx, command, embed).await;
        }
    }
}

async fn handle_stats(ctx: &Context, command: &CommandInteraction) {
    let guild_id = match command.guild_id {
        Some(id) => id,
        None => return,
    };

    // Sub-feature gate : weekly_report_enabled (toggle UI sous audit-bot).
    if !crate::shared::discord_helpers::is_feature_enabled(
        ctx, &guild_id.to_string(), "audit-bot", "weekly_report_enabled", true,
    ).await {
        let embed = info_embed("Audit -- Statistiques hebdomadaires")
            .description("Le rapport hebdomadaire est desactive pour ce serveur.");
        reply_ephemeral_embed(ctx, command, embed).await;
        return;
    }

    let data = ctx.data.read().await;

    let stats_text = if let Some(tracker) = data.get::<WeeklyTrackerKey>() {
        // Read current stats without draining
        let snapshot = tracker.snapshot(guild_id);
        format!(
            "\
Membres: +{} / -{}\n\
Bans: {}\n\
Messages supprimes: {}\n\
Messages edites: {}\n\
Changements de roles: {}\n\
Changements de channels: {}\n\
Evenements vocaux: {}\n\
Anomalies detectees: {}",
            snapshot.member_joins,
            snapshot.member_leaves,
            snapshot.bans,
            snapshot.messages_deleted,
            snapshot.messages_edited,
            snapshot.role_changes,
            snapshot.channel_changes,
            snapshot.voice_events,
            snapshot.anomalies,
        )
    } else {
        "Tracker non disponible.".to_string()
    };

    let embed = info_embed("Audit -- Statistiques hebdomadaires").description(stats_text);
    reply_ephemeral_embed(ctx, command, embed).await;
}
