use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context, CreateCommand,
    CreateCommandOption,
};
use tracing::error;

use crate::shared::discord_helpers::reply_ephemeral_embed;
use crate::shared::embeds::{moderate_embed, success_embed};
use crate::shared::heartbeat::ApiClientKey;

use super::api_client::ApiClient;

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
            reply_error(
                ctx,
                command,
                "Cette commande ne peut etre utilisee que sur un serveur.",
            )
            .await;
            return;
        }
    };

    // Verifier la permission ADMINISTRATOR
    if !has_administrator(command) {
        reply_error(
            ctx,
            command,
            "Vous n'avez pas la permission **Administrateur**.",
        )
        .await;
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
    let grpc = match data.get::<crate::shared::grpc_client::GrpcClientKey>() {
        Some(g) => g,
        None => {
            error!("GrpcClientKey manquant");
            reply_error(ctx, command, "Erreur interne : client API non disponible.").await;
            return;
        }
    };
    let api = ApiClient::new(grpc.clone());

    let guild_str = guild_id.to_string();

    match sub {
        "logs" => match api.purge_logs(jours).await {
            Ok(count) => {
                let embed = success_embed("Nettoyage des logs").description(format!(
                    "{} log(s) de plus de {} jour(s) supprime(s).",
                    count, jours
                ));
                reply_ephemeral_embed(ctx, command, embed).await;
            }
            Err(e) => {
                reply_error(ctx, command, &format!("Erreur : {}", e)).await;
            }
        },
        "infractions" => match api.purge_infractions(&guild_str, jours).await {
            Ok(count) => {
                let embed = success_embed("Nettoyage des infractions").description(format!(
                    "{} infraction(s) de plus de {} jour(s) supprimee(s).",
                    count, jours
                ));
                reply_ephemeral_embed(ctx, command, embed).await;
            }
            Err(e) => {
                reply_error(ctx, command, &format!("Erreur : {}", e)).await;
            }
        },
        "audit" => match api.purge_audit_logs(&guild_str, jours).await {
            Ok(count) => {
                let embed = success_embed("Nettoyage des logs d'audit").description(format!(
                    "{} entree(s) d'audit de plus de {} jour(s) supprimee(s).",
                    count, jours
                ));
                reply_ephemeral_embed(ctx, command, embed).await;
            }
            Err(e) => {
                reply_error(ctx, command, &format!("Erreur : {}", e)).await;
            }
        },
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
///
/// Lit les permissions fournies par Discord dans le payload d'interaction
/// (`command.member.permissions`), independantes du cache. L'ancienne version
/// passait par `to_guild_cached`, qui renvoie None quand la guild n'est pas en
/// cache -> la commande echouait pour tout le monde. Fail-closed si absent.
fn has_administrator(command: &CommandInteraction) -> bool {
    command
        .member
        .as_ref()
        .and_then(|m| m.permissions)
        .map(|p| p.administrator())
        .unwrap_or(false)
}

/// Reponse d'erreur ephemere.
async fn reply_error(ctx: &Context, command: &CommandInteraction, message: &str) {
    let embed = moderate_embed("Erreur").description(message);
    reply_ephemeral_embed(ctx, command, embed).await;
}
