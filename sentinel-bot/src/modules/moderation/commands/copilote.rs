//! Commande `/copilote membre:<@user>` — Copilote de moderation.
//!
//! Assemble une **fiche membre** (strikes, sanctions, reviews) et une
//! **suggestion de sanction proportionnee** (consultative) derivee de l'historique
//! et de la jurisprudence du serveur. Reponse EPHEMERE (donnee staff only). Le
//! bot ne fait que RENDRE ce que l'API renvoie : aucune logique de suggestion ici.

use serenity::all::{
    CommandInteraction, CommandOptionType, Context, CreateCommand, CreateCommandOption, CreateEmbed,
};
use tracing::{error, warn};

use super::api_client::MemberContext;
use super::ModerationApiKey;
use crate::shared::api_client::BaseApiClient;
use crate::shared::discord_helpers::reply_ephemeral;
use crate::shared::discord_helpers::reply_ephemeral_embed;
use crate::shared::embeds::{action_emoji, danger_embed, info_embed, moderate_embed, warn_embed};
use crate::shared::heartbeat::ApiClientKey;

pub fn register() -> CreateCommand {
    CreateCommand::new("copilote")
        .description("Fiche membre + suggestion de sanction proportionnee (consultatif)")
        .default_member_permissions(serenity::all::Permissions::MODERATE_MEMBERS)
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::User,
                "membre",
                "Membre a analyser (ou utilise user_id)",
            )
            .required(false),
        )
        .add_option(CreateCommandOption::new(
            CommandOptionType::String,
            "user_id",
            "ID du membre (ex. membre parti / banni)",
        ))
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    if !super::has_mod_permission(command, serenity::all::Permissions::MODERATE_MEMBERS) {
        reply_ephemeral(
            ctx,
            command,
            "❌ Permission MODERATE_MEMBERS requise pour /copilote.",
        )
        .await;
        warn!(user = %command.user.name, "Tentative /copilote sans permission");
        return;
    }

    let guild_id = match command.guild_id {
        Some(id) => id,
        None => {
            reply_ephemeral(ctx, command, "Commande serveur uniquement.").await;
            return;
        }
    };

    let target_id = match super::resolve_target_user_id(command, "membre") {
        Some(id) => id,
        None => {
            reply_ephemeral(
                ctx,
                command,
                "Indique un membre (`membre`) ou un identifiant (`user_id`).",
            )
            .await;
            return;
        }
    };

    // ── Lecture de la config du module (par serveur) ──
    let (enabled, lookback_days, min_precedents) = {
        let data = ctx.data.read().await;
        let Some(base) = data.get::<ApiClientKey>() else {
            error!("ApiClientKey manquant pour /copilote");
            reply_ephemeral(ctx, command, "Service indisponible, reessaie plus tard.").await;
            return;
        };
        let cfg = base
            .get_guild_config_for(
                &guild_id.to_string(),
                crate::modules::moderation::MODULE_BOT_NAME,
            )
            .await
            .unwrap_or_default();
        (
            BaseApiClient::config_bool(&cfg, "copilot_enabled", false),
            BaseApiClient::config_u64(&cfg, "copilot_lookback_days", 90),
            BaseApiClient::config_u64(&cfg, "copilot_min_precedents", 3),
        )
    };

    if !enabled {
        reply_ephemeral(
            ctx,
            command,
            "Copilote désactivé (activez-le dans la config du module modération).",
        )
        .await;
        return;
    }

    let data = ctx.data.read().await;
    let api = match data.get::<ModerationApiKey>() {
        Some(a) => a,
        None => {
            error!("ModerationApiKey manquant pour /copilote");
            return;
        }
    };

    let context = match api
        .get_member_context(
            &guild_id.to_string(),
            &target_id.to_string(),
            lookback_days as i64,
            min_precedents as u32,
        )
        .await
    {
        Ok(c) => c,
        Err(e) => {
            error!(error = %e, "Erreur recuperation copilote");
            reply_ephemeral(ctx, command, &format!("Erreur : {e}")).await;
            return;
        }
    };

    let username = target_id
        .to_user(&ctx.http)
        .await
        .map(|u| u.name)
        .unwrap_or_else(|_| "inconnu".to_string());

    let embed = build_embed(&context, target_id.get(), &username);
    reply_ephemeral_embed(ctx, command, embed).await;
}

/// Construit l'embed de la fiche copilote. Best-effort, ne panique jamais.
fn build_embed(ctx: &MemberContext, user_id: u64, username: &str) -> CreateEmbed {
    // Couleur/emoji coherents avec la gravite de l'action suggeree.
    let base = match ctx.suggestion.action.as_deref() {
        Some("ban") => danger_embed(format!("🧭 Copilote — @{username}")),
        Some("mute") | Some("delete") => moderate_embed(format!("🧭 Copilote — @{username}")),
        Some("warn") => warn_embed(format!("🧭 Copilote — @{username}")),
        _ => info_embed(format!("🧭 Copilote — @{username}")),
    };

    // ── Fiche membre ──
    let sanctions = if ctx.sanctions_by_type.is_empty() {
        "Aucune".to_string()
    } else {
        ctx.sanctions_by_type
            .iter()
            .map(|c| format!("{} {} ×{}", action_emoji(&c.action), c.action, c.count))
            .collect::<Vec<_>>()
            .join(" · ")
    };

    let last_sanction = match ctx.last_sanction_at.as_deref() {
        Some(rfc) => chrono::DateTime::parse_from_rfc3339(rfc)
            .map(|d| format!("<t:{}:R>", d.timestamp()))
            .unwrap_or_else(|_| "—".to_string()),
        None => "—".to_string(),
    };

    let fiche = format!(
        "👤 <@{user_id}>\n\
         🎯 Strikes actifs : **{}**\n\
         📋 Sanctions : {}\n\
         🕒 Dernière sanction : {}\n\
         🔎 Reviews automod ouvertes : **{}**",
        ctx.active_strikes, sanctions, last_sanction, ctx.open_reviews
    );

    // ── Jurisprudence ──
    let jurisprudence = if ctx.precedents.total == 0 || ctx.precedents.counts_by_action.is_empty() {
        "Aucun précédent".to_string()
    } else {
        let dist = ctx
            .precedents
            .counts_by_action
            .iter()
            .map(|c| format!("{} {}", c.count, c.action))
            .collect::<Vec<_>>()
            .join(", ");
        let cat = if ctx.precedents.flag_category.is_empty() {
            "catégorie inconnue"
        } else {
            ctx.precedents.flag_category.as_str()
        };
        format!("{cat} : {dist}")
    };

    // ── Suggestion (consultative) ──
    let precedents_note = if ctx.suggestion.precedent_count > 0 {
        format!(" _(sur {} précédent(s))_", ctx.suggestion.precedent_count)
    } else {
        String::new()
    };
    let suggestion = match (&ctx.suggestion.action, ctx.suggestion.basis.as_str()) {
        (Some(action), basis) if basis != "insufficient" => format!(
            "{} **{}**{}\n{}\n\n_Consultatif — tu gardes la décision._",
            action_emoji(action),
            action,
            precedents_note,
            ctx.suggestion.rationale
        ),
        _ => format!(
            "Aucune suggestion.\n{}\n\n_Consultatif — tu gardes la décision._",
            ctx.suggestion.rationale
        ),
    };

    base.field("Fiche", fiche, false)
        .field("Jurisprudence", jurisprudence, false)
        .field("Suggestion", suggestion, false)
}
