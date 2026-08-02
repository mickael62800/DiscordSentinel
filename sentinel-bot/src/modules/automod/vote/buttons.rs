//! Handlers des boutons de vote, de cloture et de reouverture.

use serenity::prelude::*;
use tracing::{info, warn};

use crate::shared::api_client::BaseApiClient;
use crate::shared::heartbeat::ApiClientKey;

use super::super::detectors;
use super::super::review;
use super::cards::{archive_discussion_channel, reply_ephemeral};
use super::labels::{action_label, char_to_str};
use super::render::{
    build_detail_url, moderator_facts, render_history_totals, render_votes, reopen_row,
    secondary_row, vote_buttons, vote_embed, VoteDto, VOTES_FIELD,
};
use super::{CLOSE_PREFIX, REOPEN_PREFIX, VOTE_PREFIX};

/// Handler du bouton de vote (`amv:<char>:<review_id>`).
pub(crate) async fn handle_vote_button(
    ctx: &Context,
    component: &serenity::model::application::ComponentInteraction,
) {
    let rest = match component.data.custom_id.strip_prefix(VOTE_PREFIX) {
        Some(r) => r,
        None => return,
    };
    let (char_part, review_id) = match rest.split_once(':') {
        Some((c, id)) => (c, id),
        None => return,
    };
    let vote_action = char_to_str(char_part.chars().next().unwrap_or('i'));

    let guild_id = component
        .guild_id
        .map(|g| g.to_string())
        .unwrap_or_default();
    let api = {
        let data = ctx.data.read().await;
        match data.get::<ApiClientKey>() {
            Some(a) => a.clone(),
            None => return,
        }
    };
    let config = api
        .get_guild_config_for(&guild_id, super::super::MODULE_BOT_NAME)
        .await
        .unwrap_or_default();
    let mod_role_id = config
        .get("vote_mod_role_id")
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|x| *x > 0);

    // Faits Discord du votant -> la regle d'acces (is_moderator) est appliquee
    // cote API/domaine (full hexa). Un refus revient en erreur (403).
    let facts = moderator_facts(component, mod_role_id, None);
    let body = serde_json::json!({
        "voter_id": component.user.id.to_string(),
        "voter_name": component.user.name,
        "vote_action": vote_action,
        "is_admin": facts.0,
        "has_moderate_members": facts.1,
        "has_manage_messages": facts.2,
        "has_mod_role": facts.3,
    });
    let votes: Vec<VoteDto> = match api
        .post_json(&format!("/api/automod/reviews/{review_id}/vote"), &body)
        .await
    {
        Ok(v) => v,
        Err(e) => {
            warn!(error = %e, review_id, "Vote refuse ou echec");
            reply_ephemeral(
                ctx,
                component,
                "Vote impossible (non autorise ou vote clos).",
            )
            .await;
            return;
        }
    };

    // Reconstruit proprement la carte : on recopie tous les champs existants
    // SAUF l'ancien champ "Votes", puis on ajoute le decompte nominatif a jour.
    if let Some(existing) = component.message.embeds.first() {
        let mut rebuilt = serenity::builder::CreateEmbed::new()
            .color(existing.colour.map(|c| c.0).unwrap_or(0x5865f2))
            .timestamp(serenity::model::Timestamp::now());
        if let Some(title) = &existing.title {
            rebuilt = rebuilt.title(title.clone());
        }
        if let Some(thumb) = &existing.thumbnail {
            rebuilt = rebuilt.thumbnail(thumb.url.clone());
        }
        if let Some(footer) = &existing.footer {
            rebuilt = rebuilt.footer(serenity::builder::CreateEmbedFooter::new(
                footer.text.clone(),
            ));
        }
        for f in &existing.fields {
            if f.name != VOTES_FIELD {
                rebuilt = rebuilt.field(f.name.clone(), f.value.clone(), f.inline);
            }
        }
        rebuilt = rebuilt.field(VOTES_FIELD, render_votes(&votes), false);
        let _ = component
            .create_response(
                &ctx.http,
                serenity::builder::CreateInteractionResponse::UpdateMessage(
                    serenity::builder::CreateInteractionResponseMessage::new().embed(rebuilt),
                ),
            )
            .await;
    } else {
        reply_ephemeral(
            ctx,
            component,
            &format!("Vote enregistre : {}.", action_label(vote_action)),
        )
        .await;
    }
    info!(review_id, voter = %component.user.name, vote = vote_action, "Vote automod enregistre");
}

/// Handler du bouton "Clore (ignorer)" (`amclose:<review_id>`).
/// Tout moderateur peut clore immediatement : statut -> ignored, aucune
/// sanction, carte mise a jour + bouton "Rouvrir le dossier".
pub(crate) async fn handle_close_button(
    ctx: &Context,
    component: &serenity::model::application::ComponentInteraction,
) {
    let review_id = match component.data.custom_id.strip_prefix(CLOSE_PREFIX) {
        Some(r) => r.to_string(),
        None => return,
    };
    let guild_id = component
        .guild_id
        .map(|g| g.to_string())
        .unwrap_or_default();
    let api = {
        let data = ctx.data.read().await;
        match data.get::<ApiClientKey>() {
            Some(a) => a.clone(),
            None => return,
        }
    };
    let config = api
        .get_guild_config_for(&guild_id, super::super::MODULE_BOT_NAME)
        .await
        .unwrap_or_default();
    let mod_role_id = config
        .get("vote_mod_role_id")
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|x| *x > 0);
    let facts = moderator_facts(component, mod_role_id, None);

    let body = serde_json::json!({
        "actor_id": component.user.id.to_string(),
        "actor_name": component.user.name,
        "source": "discord",
        "is_admin": facts.0,
        "has_moderate_members": facts.1,
        "has_manage_messages": facts.2,
        "has_mod_role": facts.3,
    });
    if let Err(e) = api
        .post_json::<_, serde_json::Value>(
            &format!("/api/automod/reviews/{review_id}/ignore"),
            &body,
        )
        .await
    {
        warn!(error = %e, review_id, "Echec clore (ignorer) : non autorise ou deja clos");
        reply_ephemeral(
            ctx,
            component,
            "Cloture impossible (reserve aux moderateurs, ou deja clos).",
        )
        .await;
        return;
    }

    // Recupere la review (membre + infraction) pour garder une carte lisible,
    // et archive le salon de discussion lie.
    #[derive(serde::Deserialize, Default)]
    struct ClosedReview {
        #[serde(default)]
        user_id: String,
        #[serde(default)]
        user_name: String,
        #[serde(default)]
        content_preview: String,
        #[serde(default)]
        reason: String,
        #[serde(default)]
        incident_count: i32,
    }
    let r: ClosedReview = api
        .get_json(&format!("/api/automod/reviews/{review_id}"))
        .await
        .unwrap_or_default();
    if !r.user_id.is_empty() {
        archive_discussion_channel(ctx, &api, &review_id, &r.user_id).await;
    }

    let mut closed = serenity::builder::CreateEmbed::new()
        .title("AutoMod -- Dossier clos (ignore)")
        .description(format!(
            "Clos par **{}**. Aucune sanction appliquee.\nUn moderateur peut rouvrir le dossier si besoin.",
            component.user.name
        ))
        .color(0x95a5a6)
        .field("Membre", format!("<@{}> (`{}`)", r.user_id, r.user_name), true)
        .field("Raison", if r.reason.is_empty() { "—" } else { r.reason.as_str() }, false)
        .field(
            if r.incident_count > 1 { "Dernier message" } else { "Message" },
            format!("```{}```", review::sanitize_embed_content(&r.content_preview, 500)),
            false,
        );
    if r.incident_count > 1 {
        closed = closed.field("Incidents", r.incident_count.to_string(), true);
    }
    closed = closed
        .footer(serenity::builder::CreateEmbedFooter::new(
            "AutoMod | Dossier ignore",
        ))
        .timestamp(serenity::model::Timestamp::now());
    let _ = component
        .create_response(
            &ctx.http,
            serenity::builder::CreateInteractionResponse::UpdateMessage(
                serenity::builder::CreateInteractionResponseMessage::new()
                    .embed(closed)
                    .components(vec![reopen_row(&review_id)]),
            ),
        )
        .await;
    info!(review_id, moderator = %component.user.name, "Dossier automod clos (ignore)");
}

/// Handler du bouton "Rouvrir le dossier" (`amreopen:<review_id>`).
/// Repasse la review en vote (nouvelle echeance) et reconstruit la carte de
/// vote complete. Tout moderateur peut rouvrir.
pub(crate) async fn handle_reopen_button(
    ctx: &Context,
    component: &serenity::model::application::ComponentInteraction,
) {
    let review_id = match component.data.custom_id.strip_prefix(REOPEN_PREFIX) {
        Some(r) => r.to_string(),
        None => return,
    };
    let guild_id = component
        .guild_id
        .map(|g| g.to_string())
        .unwrap_or_default();
    let api = {
        let data = ctx.data.read().await;
        match data.get::<ApiClientKey>() {
            Some(a) => a.clone(),
            None => return,
        }
    };
    let config = api
        .get_guild_config_for(&guild_id, super::super::MODULE_BOT_NAME)
        .await
        .unwrap_or_default();
    let mod_role_id = config
        .get("vote_mod_role_id")
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|x| *x > 0);
    let deadline_hours = BaseApiClient::config_u64(&config, "vote_deadline_hours", 72) as i64;
    let discussion_enabled =
        BaseApiClient::config_bool(&config, "discussion_channel_enabled", false);
    let detail_url = build_detail_url(&config, &guild_id);
    let facts = moderator_facts(component, mod_role_id, None);

    #[derive(serde::Deserialize)]
    struct ReopenedReview {
        guild_id: String,
        channel_id: String,
        message_id: String,
        user_id: String,
        user_name: String,
        content_preview: String,
        suggested_action: String,
        score: f64,
        reason: String,
        flags: serde_json::Value,
        voting_deadline: Option<String>,
    }

    let body = serde_json::json!({
        "actor_id": component.user.id.to_string(),
        "actor_name": component.user.name,
        "deadline_hours": deadline_hours,
        "source": "discord",
        "is_admin": facts.0,
        "has_moderate_members": facts.1,
        "has_manage_messages": facts.2,
        "has_mod_role": facts.3,
    });
    let review: ReopenedReview = match api
        .post_json(&format!("/api/automod/reviews/{review_id}/reopen"), &body)
        .await
    {
        Ok(r) => r,
        Err(e) => {
            warn!(error = %e, review_id, "Echec rouvrir : non autorise ou deja ouvert");
            reply_ephemeral(
                ctx,
                component,
                "Reouverture impossible (reserve aux moderateurs, ou deja ouvert).",
            )
            .await;
            return;
        }
    };

    let flags = detectors::DetectionFlags {
        spam: review
            .flags
            .get("spam")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        insult: review
            .flags
            .get("insult")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        profanity: review
            .flags
            .get("profanity")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        link: review
            .flags
            .get("link")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        phishing: review
            .flags
            .get("phishing")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
    };
    let deadline = review
        .voting_deadline
        .as_deref()
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|d| d.with_timezone(&chrono::Utc))
        .unwrap_or_else(|| chrono::Utc::now() + chrono::Duration::hours(deadline_hours));

    let mut embed = vote_embed(
        &review.user_id,
        &review.user_name,
        &review.channel_id,
        review.score,
        &review.content_preview,
        &review.reason,
        &flags,
        &review.suggested_action,
        &deadline,
        &[],
    );
    if let Some(hist) = render_history_totals(ctx, &review.guild_id, &review.user_id).await {
        embed = embed.field("📋 Antecedents du membre", hist, false);
    }
    embed = embed.field(
        "♻️ Dossier rouvert",
        format!(
            "Rouvert par **{}** — nouveau vote en cours.",
            component.user.name
        ),
        false,
    );

    let msg_url = format!(
        "https://discord.com/channels/{}/{}/{}",
        review.guild_id, review.channel_id, review.message_id
    );
    let link_row = secondary_row(
        &msg_url,
        &review_id,
        discussion_enabled,
        detail_url.as_deref(),
    );
    let mut rows = vote_buttons(&review_id);
    rows.push(link_row);

    let _ = component
        .create_response(
            &ctx.http,
            serenity::builder::CreateInteractionResponse::UpdateMessage(
                serenity::builder::CreateInteractionResponseMessage::new()
                    .embed(embed)
                    .components(rows),
            ),
        )
        .await;
    info!(review_id, moderator = %component.user.name, "Dossier automod rouvert (vote relance)");
}
