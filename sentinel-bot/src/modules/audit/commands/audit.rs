use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context, CreateCommand,
    CreateCommandOption,
};

use crate::shared::discord_helpers::reply_ephemeral_embed;
use crate::shared::embeds::info_embed;

use super::api_client::ApiClient;

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
                CreateCommandOption::new(CommandOptionType::User, "user", "Utilisateur cible")
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
        .add_option(CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "stats",
            "Affiche les statistiques hebdomadaires",
        ))
}

/// Verifie (fail-closed) que l'appelant a MODERATE_MEMBERS (ou ADMINISTRATOR).
fn has_mod_permission(command: &CommandInteraction) -> bool {
    use serenity::all::Permissions;
    command
        .member
        .as_ref()
        .and_then(|m| m.permissions)
        .map(|p| {
            p.contains(Permissions::MODERATE_MEMBERS) || p.contains(Permissions::ADMINISTRATOR)
        })
        .unwrap_or(false)
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    if !has_mod_permission(command) {
        let embed = info_embed("Audit")
            .description("\u{274c} Permission de moderation requise pour cette commande.");
        reply_ephemeral_embed(ctx, command, embed).await;
        return;
    }

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

    let target_id = sub_opts.iter().find(|o| o.name == "user").and_then(|o| {
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
    let base = match data.get::<crate::shared::grpc_client::GrpcClientKey>() {
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
                        let actor = e.actor_name.as_deref().unwrap_or("?");
                        let target = e.target_name.as_deref().unwrap_or("?");
                        format!(
                            "{}. **{}** par {} sur {}",
                            i + 1,
                            e.event_type,
                            actor,
                            target
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            };

            let embed = info_embed("Audit -- Recherche").description(description);
            reply_ephemeral_embed(ctx, command, embed).await;
        }
        Err(e) => {
            let embed = info_embed("Audit -- Recherche").description(format!("Erreur : {}", e));
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
        ctx,
        &guild_id.to_string(),
        "audit-bot",
        "weekly_report_enabled",
        true,
    )
    .await
    {
        let embed = info_embed("Audit -- Statistiques hebdomadaires")
            .description("Le rapport hebdomadaire est desactive pour ce serveur.");
        reply_ephemeral_embed(ctx, command, embed).await;
        return;
    }

    // Agregation server-side : l'API compte les events d'audit persistes sur
    // 7 jours. Le bot ne fait que rendre l'embed a partir de ces compteurs.
    let base = {
        let data = ctx.data.read().await;
        match data.get::<crate::shared::grpc_client::GrpcClientKey>() {
            Some(a) => a.clone(),
            None => return,
        }
    };
    let api = ApiClient::new(base);

    let stats_text = match api.get_weekly_report(&guild_id.to_string()).await {
        Ok(report) => format!(
            "\
Membres: +{} / -{}\n\
Bans: {}\n\
Messages supprimes: {}\n\
Messages edites: {}\n\
Changements de roles: {}\n\
Changements de channels: {}\n\
Evenements vocaux: {}\n\
Anomalies detectees: {}",
            report.member_joins,
            report.member_leaves,
            report.bans,
            report.messages_deleted,
            report.messages_edited,
            report.role_changes,
            report.channel_changes,
            report.voice_events,
            report.anomalies,
        ),
        Err(e) => format!("Erreur recuperation du rapport : {}", e),
    };

    let embed = info_embed("Audit -- Statistiques hebdomadaires").description(stats_text);
    reply_ephemeral_embed(ctx, command, embed).await;
}
