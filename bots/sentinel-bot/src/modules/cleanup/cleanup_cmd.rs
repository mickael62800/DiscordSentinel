use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context,
    CreateCommand, CreateCommandOption,
};
use tracing::error;

use sentinel_shared::discord_helpers::reply_ephemeral_embed;
use sentinel_shared::embeds::{success_embed, moderate_embed};
use sentinel_shared::heartbeat::ApiClientKey;

use crate::api_client::ApiClient;

pub fn register() -> CreateCommand {
    CreateCommand::new("cleanup")
        .description("Nettoyer les donnees anciennes (admin)")
        .default_member_permissions(serenity::all::Permissions::ADMINISTRATOR)
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "logs",
                "Purger les logs systeme plus anciens que X jours",
            )
            .add_sub_option(
                CreateCommandOption::new(
                    CommandOptionType::Integer,
                    "jours",
                    "Nombre de jours a conserver",
                )
                .min_int_value(1)
                .max_int_value(365)
                .required(true),
            ),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "infractions",
                "Purger les infractions plus anciennes que X jours",
            )
            .add_sub_option(
                CreateCommandOption::new(
                    CommandOptionType::Integer,
                    "jours",
                    "Nombre de jours a conserver",
                )
                .min_int_value(1)
                .max_int_value(365)
                .required(true),
            ),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "audit",
                "Purger les logs d'audit plus anciens que X jours",
            )
            .add_sub_option(
                CreateCommandOption::new(
                    CommandOptionType::Integer,
                    "jours",
                    "Nombre de jours a conserver",
                )
                .min_int_value(1)
                .max_int_value(365)
                .required(true),
            ),
        )
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    let guild_id = match command.guild_id {
        Some(id) => id,
        None => {
            reply_error(ctx, command, "Cette commande ne peut etre utilisee que sur un serveur.").await;
            return;
        }
    };

    // Verifier la permission ADMINISTRATOR
    if !has_administrator(ctx, command).await {
        reply_error(ctx, command, "Vous n'avez pas la permission **Administrateur**.").await;
        return;
    }

    let sub = command
        .data
        .options
        .first()
        .map(|o| o.name.as_str())
        .unwrap_or("");

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

    let jours = sub_opts
        .iter()
        .find(|o| o.name == "jours")
        .and_then(|o| o.value.as_i64())
        .unwrap_or(30) as u64;

    let data = ctx.data.read().await;
    let base = match data.get::<ApiClientKey>() {
        Some(a) => a,
        None => {
            error!("ApiClientKey manquant");
            reply_error(ctx, command, "Erreur interne : client API non disponible.").await;
            return;
        }
    };
    let api = ApiClient::new(base.clone());

    let guild_str = guild_id.to_string();

    match sub {
        "logs" => {
            match api.purge_logs(jours).await {
                Ok(count) => {
                    let embed = success_embed("Nettoyage des logs")
                        .description(format!(
                            "{} log(s) de plus de {} jour(s) supprime(s).",
                            count, jours
                        ));
                    reply_ephemeral_embed(ctx, command, embed).await;
                }
                Err(e) => {
                    reply_error(ctx, command, &format!("Erreur : {}", e)).await;
                }
            }
        }
        "infractions" => {
            match api.purge_infractions(&guild_str, jours).await {
                Ok(count) => {
                    let embed = success_embed("Nettoyage des infractions")
                        .description(format!(
                            "{} infraction(s) de plus de {} jour(s) supprimee(s).",
                            count, jours
                        ));
                    reply_ephemeral_embed(ctx, command, embed).await;
                }
                Err(e) => {
                    reply_error(ctx, command, &format!("Erreur : {}", e)).await;
                }
            }
        }
        "audit" => {
            match api.purge_audit_logs(&guild_str, jours).await {
                Ok(count) => {
                    let embed = success_embed("Nettoyage des logs d'audit")
                        .description(format!(
                            "{} entree(s) d'audit de plus de {} jour(s) supprimee(s).",
                            count, jours
                        ));
                    reply_ephemeral_embed(ctx, command, embed).await;
                }
                Err(e) => {
                    reply_error(ctx, command, &format!("Erreur : {}", e)).await;
                }
            }
        }
        _ => {
            reply_error(ctx, command, "Sous-commande inconnue.").await;
        }
    }

    // Log via API
    if let Some(api) = data.get::<ApiClientKey>() {
        api.send_log(
            "info",
            &guild_str,
            &format!(
                "Cleanup {} : {} jour(s) par {}",
                sub, jours, command.user.name
            ),
        );
    }
}

/// Verifie si l'utilisateur a la permission ADMINISTRATOR.
async fn has_administrator(ctx: &Context, command: &CommandInteraction) -> bool {
    let guild_id = match command.guild_id {
        Some(id) => id,
        None => return false,
    };

    match guild_id.member(&ctx.http, command.user.id).await {
        Ok(member) => {
            if let Some(guild) = guild_id.to_guild_cached(&ctx.cache) {
                let permissions = guild.member_permissions(&member);
                permissions.administrator()
            } else {
                false
            }
        }
        Err(_) => false,
    }
}

/// Reponse d'erreur ephemere.
async fn reply_error(ctx: &Context, command: &CommandInteraction, message: &str) {
    let embed = moderate_embed("Erreur").description(message);
    reply_ephemeral_embed(ctx, command, embed).await;
}
