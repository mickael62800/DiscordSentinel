use std::time::Duration;

use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context,
    CreateCommand, CreateCommandOption, GetMessages, MessageId,
};
use tracing::error;

use crate::shared::discord_helpers::{defer_ephemeral, followup_ephemeral_embed};
use crate::shared::embeds::{success_embed, moderate_embed};
use crate::shared::heartbeat::ApiClientKey;

/// Limite Discord : bulk_delete ne fonctionne que sur les messages < 14 jours.
const DISCORD_BULK_DELETE_MAX_AGE_SECS: i64 = 14 * 24 * 60 * 60;
/// Taille de batch pour bulk_delete Discord (max 100 par appel API Discord).
const DISCORD_BULK_DELETE_BATCH: usize = 100;
/// Delai entre suppressions individuelles (rate limit Discord).
const DISCORD_DELETE_RATE_LIMIT_MS: u64 = 300;

pub fn register() -> CreateCommand {
    CreateCommand::new("purge")
        .description("Supprimer des messages dans le salon")
        .default_member_permissions(serenity::all::Permissions::MANAGE_MESSAGES)
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "last",
                "Supprimer les X derniers messages",
            )
            .add_sub_option(
                CreateCommandOption::new(
                    CommandOptionType::Integer,
                    "nombre",
                    "Nombre de messages a supprimer (1-100)",
                )
                .min_int_value(1)
                .max_int_value(100)
                .required(true),
            ),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "user",
                "Supprimer les messages d'un utilisateur (par membre OU par ID)",
            )
            .add_sub_option(
                CreateCommandOption::new(
                    CommandOptionType::User,
                    "utilisateur",
                    "Membre cible (laisse vide si tu utilises user_id)",
                ),
            )
            .add_sub_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "user_id",
                    "ID de l'utilisateur (utile pour un membre parti / banni)",
                ),
            )
            .add_sub_option(
                CreateCommandOption::new(
                    CommandOptionType::Integer,
                    "nombre",
                    "Nombre de messages RECENTS a analyser dans le salon (1-100)",
                )
                .min_int_value(1)
                .max_int_value(100)
                .required(true),
            ),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "contains",
                "Supprimer les messages contenant un texte",
            )
            .add_sub_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "texte",
                    "Texte a rechercher",
                )
                .required(true),
            )
            .add_sub_option(
                CreateCommandOption::new(
                    CommandOptionType::Integer,
                    "nombre",
                    "Nombre de messages a analyser (1-100)",
                )
                .min_int_value(1)
                .max_int_value(100)
                .required(true),
            ),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "bots",
                "Supprimer les messages de bots",
            )
            .add_sub_option(
                CreateCommandOption::new(
                    CommandOptionType::Integer,
                    "nombre",
                    "Nombre de messages a analyser (1-100)",
                )
                .min_int_value(1)
                .max_int_value(100)
                .required(true),
            ),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "links",
                "Supprimer les messages contenant des liens",
            )
            .add_sub_option(
                CreateCommandOption::new(
                    CommandOptionType::Integer,
                    "nombre",
                    "Nombre de messages a analyser (1-100)",
                )
                .min_int_value(1)
                .max_int_value(100)
                .required(true),
            ),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "attachments",
                "Supprimer les messages avec des fichiers joints",
            )
            .add_sub_option(
                CreateCommandOption::new(
                    CommandOptionType::Integer,
                    "nombre",
                    "Nombre de messages a analyser (1-100)",
                )
                .min_int_value(1)
                .max_int_value(100)
                .required(true),
            ),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "all",
                "Supprimer TOUS les messages du salon (peut etre long)",
            )
            .add_sub_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "confirmation",
                    "Tapez CONFIRMER (en majuscules) pour valider cette action irreversible",
                )
                .required(true),
            ),
        )
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    // Defer immediatement : /purge peut depasser 3s (fetch + delete par lots avec sleep)
    defer_ephemeral(ctx, command).await;

    let guild_id = match command.guild_id {
        Some(id) => id,
        None => {
            reply_error(ctx, command, "Cette commande ne peut etre utilisee que sur un serveur.").await;
            return;
        }
    };

    // Verifier la permission MANAGE_MESSAGES
    if !has_manage_messages(ctx, command).await {
        reply_error(ctx, command, "Vous n'avez pas la permission **Gerer les messages**.").await;
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

    let nombre = sub_opts
        .iter()
        .find(|o| o.name == "nombre")
        .and_then(|o| o.value.as_i64())
        .unwrap_or(10)
        .min(100) as u8;

    let channel_id = command.channel_id;

    // Branche speciale : /purge all — paginate jusqu'a vider le salon
    if sub == "all" {
        // Garde-fou irreversible : on exige le texte "CONFIRMER" en clair
        // plutot qu'un simple clic. Evite les erreurs de manipulation.
        let confirmation = sub_opts
            .iter()
            .find(|o| o.name == "confirmation")
            .and_then(|o| o.value.as_str())
            .unwrap_or("");
        if confirmation != "CONFIRMER" {
            reply_error(
                ctx,
                command,
                "Action annulee. Pour confirmer la suppression **irreversible** de tous les messages, \
                 tapez exactement `CONFIRMER` (en majuscules) dans le parametre `confirmation`.",
            )
            .await;
            return;
        }

        let (deleted, errors) = purge_all(ctx, channel_id).await;
        let description = if errors > 0 {
            format!(
                "{} message(s) supprime(s).\n{} erreur(s) rencontree(s).",
                deleted, errors
            )
        } else {
            format!("{} message(s) supprime(s).", deleted)
        };
        let embed = success_embed("Purge complete terminee").description(description);
        followup_ephemeral_embed(ctx, command, embed).await;

        let data = ctx.data.read().await;
        if let Some(api) = data.get::<ApiClientKey>() {
            api.send_log(
                "info",
                &guild_id.to_string(),
                &format!(
                    "Purge all : {} message(s) supprime(s) par {}",
                    deleted, command.user.name
                ),
            );
        }
        return;
    }

    // Recuperer les messages
    let messages = match channel_id
        .messages(&ctx.http, GetMessages::new().limit(nombre))
        .await
    {
        Ok(msgs) => msgs,
        Err(e) => {
            error!(error = %e, "Erreur recuperation messages");
            reply_error(ctx, command, "Erreur lors de la recuperation des messages.").await;
            return;
        }
    };

    // Filtrer selon la sous-commande
    let filtered: Vec<_> = match sub {
        "last" => messages,
        "user" => {
            // Cible : soit le selecteur de membre, soit un ID brut (user_id)
            // — l'ID marche meme pour un membre parti ou banni.
            let from_picker = sub_opts
                .iter()
                .find(|o| o.name == "utilisateur")
                .and_then(|o| match &o.value {
                    CommandDataOptionValue::User(id) => Some(*id),
                    _ => None,
                });
            let from_id = sub_opts
                .iter()
                .find(|o| o.name == "user_id")
                .and_then(|o| o.value.as_str())
                .map(|s| s.trim().trim_start_matches("<@").trim_start_matches('!').trim_end_matches('>'))
                .and_then(|s| s.parse::<u64>().ok())
                .map(serenity::all::UserId::new);
            match from_picker.or(from_id) {
                Some(uid) => messages.into_iter().filter(|m| m.author.id == uid).collect(),
                None => {
                    reply_error(ctx, command, "Indique un membre (`utilisateur`) **ou** un identifiant (`user_id`).").await;
                    return;
                }
            }
        }
        "contains" => {
            let texte = sub_opts
                .iter()
                .find(|o| o.name == "texte")
                .and_then(|o| o.value.as_str())
                .unwrap_or("");
            let texte_lower = texte.to_lowercase();
            messages
                .into_iter()
                .filter(|m| m.content.to_lowercase().contains(&texte_lower))
                .collect()
        }
        "bots" => messages.into_iter().filter(|m| m.author.bot).collect(),
        "links" => messages
            .into_iter()
            .filter(|m| m.content.contains("http://") || m.content.contains("https://"))
            .collect(),
        "attachments" => messages
            .into_iter()
            .filter(|m| !m.attachments.is_empty())
            .collect(),
        _ => {
            reply_error(ctx, command, "Sous-commande inconnue.").await;
            return;
        }
    };

    if filtered.is_empty() {
        reply_error(ctx, command, "Aucun message correspondant trouve.").await;
        return;
    }

    // Separer les messages recents (< 14 jours) des anciens
    let now = chrono_now_unix();
    let fourteen_days_secs = DISCORD_BULK_DELETE_MAX_AGE_SECS;

    let mut recent_ids: Vec<MessageId> = Vec::new();
    let mut old_ids: Vec<MessageId> = Vec::new();

    for msg in &filtered {
        let msg_ts = msg.timestamp.unix_timestamp();
        if now - msg_ts < fourteen_days_secs {
            recent_ids.push(msg.id);
        } else {
            old_ids.push(msg.id);
        }
    }

    let total = filtered.len();
    let mut deleted = 0u64;
    let mut errors = 0u64;

    // Suppression en masse des messages recents (par lots de 100)
    for chunk in recent_ids.chunks(DISCORD_BULK_DELETE_BATCH) {
        if chunk.len() == 1 {
            // bulk_delete requiert au moins 2 messages
            if let Err(e) = channel_id.delete_message(&ctx.http, chunk[0]).await {
                error!(error = %e, "Erreur suppression message individuel");
                errors += 1;
            } else {
                deleted += 1;
            }
        } else {
            match channel_id.delete_messages(&ctx.http, chunk).await {
                Ok(_) => deleted += chunk.len() as u64,
                Err(e) => {
                    error!(error = %e, "Erreur suppression en masse, tentative individuelle");
                    // Fallback : suppression individuelle
                    for &id in chunk {
                        if let Err(e) = channel_id.delete_message(&ctx.http, id).await {
                            error!(error = %e, "Erreur suppression message");
                            errors += 1;
                        } else {
                            deleted += 1;
                        }
                        tokio::time::sleep(Duration::from_millis(DISCORD_DELETE_RATE_LIMIT_MS)).await;
                    }
                }
            }
        }
    }

    // Suppression individuelle des anciens messages (> 14 jours)
    for &id in &old_ids {
        if let Err(e) = channel_id.delete_message(&ctx.http, id).await {
            error!(error = %e, "Erreur suppression ancien message");
            errors += 1;
        } else {
            deleted += 1;
        }
        // Rate limit
        tokio::time::sleep(Duration::from_millis(DISCORD_DELETE_RATE_LIMIT_MS)).await;
    }

    // Reponse embed
    let description = if errors > 0 {
        format!(
            "{} message(s) supprime(s) sur {} trouve(s).\n{} erreur(s) rencontree(s).",
            deleted, total, errors
        )
    } else {
        format!(
            "{} message(s) supprime(s) sur {} trouve(s).",
            deleted, total
        )
    };

    let embed = success_embed("Purge terminee").description(description);
    followup_ephemeral_embed(ctx, command, embed).await;

    // Log via API
    let data = ctx.data.read().await;
    if let Some(api) = data.get::<ApiClientKey>() {
        api.send_log(
            "info",
            &guild_id.to_string(),
            &format!(
                "Purge {} : {} message(s) supprime(s) par {}",
                sub, deleted, command.user.name
            ),
        );
    }
}

/// Verifie si l'utilisateur a la permission MANAGE_MESSAGES.
async fn has_manage_messages(ctx: &Context, command: &CommandInteraction) -> bool {
    let guild_id = match command.guild_id {
        Some(id) => id,
        None => return false,
    };

    match guild_id.member(&ctx.http, command.user.id).await {
        Ok(member) => {
            if let Some(guild) = guild_id.to_guild_cached(&ctx.cache) {
                let permissions = guild.member_permissions(&member);
                permissions.manage_messages() || permissions.administrator()
            } else {
                false
            }
        }
        Err(_) => false,
    }
}

/// Retourne le timestamp Unix actuel.
fn chrono_now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

/// Purge complete d'un salon : boucle de fetch + delete jusqu'a l'epuisement.
/// Retourne (nb_supprimes, nb_erreurs).
async fn purge_all(ctx: &Context, channel_id: serenity::all::ChannelId) -> (u64, u64) {
    let mut deleted: u64 = 0;
    let mut errors: u64 = 0;
    let fourteen_days_secs = DISCORD_BULK_DELETE_MAX_AGE_SECS;
    // Garde-fou : eviter une boucle infinie si un message refuse systematiquement de mourir
    let mut empty_streak = 0u32;

    loop {
        let messages = match channel_id
            .messages(&ctx.http, GetMessages::new().limit(DISCORD_BULK_DELETE_BATCH as u8))
            .await
        {
            Ok(m) => m,
            Err(e) => {
                error!(error = %e, "Erreur fetch messages (purge all)");
                break;
            }
        };

        if messages.is_empty() {
            break;
        }

        let before = deleted;
        let now = chrono_now_unix();
        let mut recent_ids: Vec<MessageId> = Vec::new();
        let mut old_ids: Vec<MessageId> = Vec::new();
        for msg in &messages {
            if now - msg.timestamp.unix_timestamp() < fourteen_days_secs {
                recent_ids.push(msg.id);
            } else {
                old_ids.push(msg.id);
            }
        }

        for chunk in recent_ids.chunks(DISCORD_BULK_DELETE_BATCH) {
            if chunk.len() == 1 {
                if let Err(e) = channel_id.delete_message(&ctx.http, chunk[0]).await {
                    error!(error = %e, "Erreur delete individuel (purge all)");
                    errors += 1;
                } else {
                    deleted += 1;
                }
            } else {
                match channel_id.delete_messages(&ctx.http, chunk).await {
                    Ok(_) => deleted += chunk.len() as u64,
                    Err(e) => {
                        error!(error = %e, "Erreur bulk delete (purge all), fallback individuel");
                        for &id in chunk {
                            if let Err(e) = channel_id.delete_message(&ctx.http, id).await {
                                error!(error = %e, "Erreur delete fallback (purge all)");
                                errors += 1;
                            } else {
                                deleted += 1;
                            }
                            tokio::time::sleep(Duration::from_millis(DISCORD_DELETE_RATE_LIMIT_MS)).await;
                        }
                    }
                }
            }
        }

        for &id in &old_ids {
            if let Err(e) = channel_id.delete_message(&ctx.http, id).await {
                error!(error = %e, "Erreur delete ancien (purge all)");
                errors += 1;
            } else {
                deleted += 1;
            }
            tokio::time::sleep(Duration::from_millis(DISCORD_DELETE_RATE_LIMIT_MS)).await;
        }

        if deleted == before {
            empty_streak += 1;
            if empty_streak >= 2 {
                break;
            }
        } else {
            empty_streak = 0;
        }
    }

    (deleted, errors)
}

/// Reponse d'erreur ephemere (via followup : on a defer au debut du handler).
async fn reply_error(ctx: &Context, command: &CommandInteraction, message: &str) {
    let embed = moderate_embed("Erreur").description(message);
    followup_ephemeral_embed(ctx, command, embed).await;
}
