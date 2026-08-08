//! Construction des embeds, rangees de boutons et DTO de la carte de vote.

use serenity::prelude::*;

use crate::shared::api_client::BaseApiClient;

use super::super::api_client::{ReviewData, ReviewVote};
use super::super::detectors;
use super::labels::action_label;
use super::{CLOSE_PREFIX, DISCUSSION_PREFIX, REOPEN_PREFIX, VOTE_PREFIX};

/// Rangees de boutons de vote (6 actions -> 2 rangees car Discord limite a 5
/// boutons par rangee). Ordre de severite : Prevention < Warn < Delete < Mute < Ban.
pub(super) fn vote_buttons(review_id: &str) -> Vec<serenity::builder::CreateActionRow> {
    use serenity::all::ButtonStyle;
    use serenity::builder::{CreateActionRow, CreateButton};
    vec![
        CreateActionRow::Buttons(vec![
            CreateButton::new(format!("{VOTE_PREFIX}p:{review_id}"))
                .label("Prevention")
                .style(ButtonStyle::Success),
            CreateButton::new(format!("{VOTE_PREFIX}w:{review_id}"))
                .label("Warn")
                .style(ButtonStyle::Secondary),
            CreateButton::new(format!("{VOTE_PREFIX}d:{review_id}"))
                .label("Delete")
                .style(ButtonStyle::Secondary),
            CreateButton::new(format!("{VOTE_PREFIX}m:{review_id}"))
                .label("Mute")
                .style(ButtonStyle::Primary),
            CreateButton::new(format!("{VOTE_PREFIX}b:{review_id}"))
                .label("Ban")
                .style(ButtonStyle::Danger),
        ]),
        CreateActionRow::Buttons(vec![
            CreateButton::new(format!("{VOTE_PREFIX}i:{review_id}"))
                .label("Ignorer (vote)")
                .style(ButtonStyle::Secondary),
            CreateButton::new(format!("{CLOSE_PREFIX}{review_id}"))
                .label("🚫 Clore (ignorer)")
                .style(ButtonStyle::Danger),
        ]),
    ]
}

/// Rangee avec uniquement le bouton "Rouvrir le dossier" (carte close).
pub(super) fn reopen_row(review_id: &str) -> serenity::builder::CreateActionRow {
    use serenity::all::ButtonStyle;
    use serenity::builder::{CreateActionRow, CreateButton};
    CreateActionRow::Buttons(vec![CreateButton::new(format!(
        "{REOPEN_PREFIX}{review_id}"
    ))
    .label("♻️ Rouvrir le dossier")
    .style(ButtonStyle::Secondary)])
}

/// Antecedents du membre en TOTAUX (carte resumee). Le detail date est dans le
/// dashboard web. `None` si l'API moderation est indisponible.
pub(crate) async fn render_history_totals(
    ctx: &Context,
    guild_id: &str,
    user_id: &str,
) -> Option<String> {
    use crate::modules::moderation::ModerationApiKey;
    let mod_api = {
        let d = ctx.data.read().await;
        d.get::<ModerationApiKey>()?.clone()
    };
    let hist = mod_api.get_history(guild_id, user_id).await.ok()?;
    Some(format!(
        "**{}** warn · **{}** mute · **{}** ban",
        hist.total_warns, hist.total_mutes, hist.total_bans
    ))
}

/// Deuxieme rangee de boutons : lien vers le message + (option) "Ouvrir une
/// discussion" si `discussion_enabled`.
/// Construit l'URL "Voir le detail" vers le dashboard a partir de la config
/// (`dashboard_base_url`). `None` si non configuree.
pub(crate) fn build_detail_url(
    cfg: &std::collections::HashMap<String, String>,
    guild_id: &str,
) -> Option<String> {
    let base = BaseApiClient::config_or(cfg, "dashboard_base_url", "");
    let base = base.trim().trim_end_matches('/');
    if base.is_empty() {
        return None;
    }
    Some(format!("{base}/automod?guild={guild_id}"))
}

pub(super) fn secondary_row(
    msg_url: &str,
    review_id: &str,
    discussion_enabled: bool,
    detail_url: Option<&str>,
) -> serenity::builder::CreateActionRow {
    use serenity::all::ButtonStyle;
    use serenity::builder::CreateButton;
    let mut buttons = vec![CreateButton::new_link(msg_url).label("Aller au message")];
    if let Some(url) = detail_url {
        buttons.push(CreateButton::new_link(url).label("📋 Voir le détail"));
    }
    if discussion_enabled {
        buttons.push(
            CreateButton::new(format!("{DISCUSSION_PREFIX}{review_id}"))
                .label("Ouvrir une discussion")
                .style(ButtonStyle::Secondary),
        );
    }
    serenity::builder::CreateActionRow::Buttons(buttons)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn vote_embed(
    user_id: &str,
    user_name: &str,
    channel_id: &str,
    score: f64,
    content_preview: &str,
    reason: &str,
    flags: &detectors::DetectionFlags,
    suggested: &str,
    deadline: &chrono::DateTime<chrono::Utc>,
    votes: &[ReviewVote],
) -> serenity::builder::CreateEmbed {
    let mut flag_parts = Vec::new();
    if flags.spam {
        flag_parts.push("Spam");
    }
    if flags.insult {
        flag_parts.push("Insulte");
    }
    if flags.link {
        flag_parts.push("Lien");
    }
    if flags.phishing {
        flag_parts.push("Phishing");
    }
    let flags_str = if flag_parts.is_empty() {
        "Aucun".to_string()
    } else {
        flag_parts.join(", ")
    };

    serenity::builder::CreateEmbed::new()
        .title("AutoMod -- VOTE des moderateurs")
        .color(0x5865f2)
        .field(
            "Utilisateur",
            format!("<@{}> (`{}`)", user_id, user_name),
            true,
        )
        .field("Salon", format!("<#{}>", channel_id), true)
        .field("Score IA", format!("{:.2}", score), true)
        .field(
            "Message original",
            format!("```{}```", content_preview),
            false,
        )
        .field("Raison IA", reason, false)
        .field("Flags", &flags_str, true)
        .field("Suggestion IA", action_label(suggested), true)
        .field("Cloture", format!("<t:{}:R>", deadline.timestamp()), true)
        .field(VOTES_FIELD, render_votes(votes), false)
        .footer(serenity::builder::CreateEmbedFooter::new(
            "Votez la sanction. A l'echeance, un admin finalise.",
        ))
        .timestamp(serenity::model::Timestamp::now())
}

/// Nom du champ embed qui contient le decompte des votes (sert a le
/// retrouver/remplacer lors d'un nouveau vote).
pub(super) const VOTES_FIELD: &str = "Votes";

/// Rendu nominatif des votes, groupes par sanction :
/// `Avertissement (2) : Alice, Bob`.
pub(super) fn render_votes(votes: &[ReviewVote]) -> String {
    if votes.is_empty() {
        return "_Aucun vote pour l'instant._".to_string();
    }
    let order = ["warn", "delete", "mute", "ban", "ignore"];
    let mut lines = Vec::new();
    for a in order {
        let voters: Vec<&str> = votes
            .iter()
            .filter(|v| v.vote_action == a)
            .map(|v| v.voter_name.as_str())
            .collect();
        if voters.is_empty() {
            continue;
        }
        lines.push(format!(
            "**{}** ({}) : {}",
            action_label(a),
            voters.len(),
            voters.join(", ")
        ));
    }
    if lines.is_empty() {
        "_Aucun vote pour l'instant._".to_string()
    } else {
        lines.join("\n")
    }
}

/// Construit l'embed d'une carte de vote AGREGEE (plusieurs incidents pour un
/// meme utilisateur). Affiche score max ET score cumule + nb d'incidents.
pub(super) fn aggregated_vote_embed(
    resp: &ReviewData,
    votes: &[ReviewVote],
) -> serenity::builder::CreateEmbed {
    let deadline_ts = resp
        .voting_deadline
        .as_deref()
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|d| d.timestamp());

    let mut embed = serenity::builder::CreateEmbed::new()
        .title("AutoMod -- VOTE (alertes regroupees)")
        .color(0x5865f2)
        .field("Utilisateur", format!("<@{}>", resp.user_id), true)
        .field("Salon", format!("<#{}>", resp.channel_id), true)
        .field("Incidents", resp.incident_count.to_string(), true)
        .field("Score max", format!("{:.2}", resp.score), true)
        .field(
            "Score cumule",
            format!("{:.2}", resp.cumulative_score),
            true,
        )
        .field(
            "Action suggeree",
            action_label(&resp.suggested_action),
            true,
        );
    if let Some(ts) = deadline_ts {
        embed = embed.field("Cloture", format!("<t:{}:R>", ts), true);
    }
    embed = embed
        .field(
            "Dernier message",
            format!("```{}```", resp.content_preview),
            false,
        )
        .field(
            "Raison",
            if resp.reason.is_empty() {
                "—"
            } else {
                resp.reason.as_str()
            },
            false,
        );
    // Le detail complet des incidents est dans le dashboard web (bouton lien).
    embed = embed
        .field(VOTES_FIELD, render_votes(votes), false)
        .footer(serenity::builder::CreateEmbedFooter::new(
            "Alertes regroupees. Votez la sanction ; a l'echeance, un admin finalise.",
        ))
        .timestamp(serenity::model::Timestamp::now());
    embed
}

/// Collecte les FAITS Discord du demandeur (permissions + appartenance aux
/// roles configures). La DECISION d'autorisation (is_moderator / can_finalize)
/// est prise cote core (full hexa). Retourne
/// `(is_admin, has_moderate_members, has_manage_messages, has_mod_role, has_admin_role)`.
pub(super) fn moderator_facts(
    component: &serenity::model::application::ComponentInteraction,
    mod_role_id: Option<u64>,
    admin_role_id: Option<u64>,
) -> (bool, bool, bool, bool, bool) {
    use serenity::all::Permissions;
    let perms = component.member.as_ref().and_then(|m| m.permissions);
    let has = |p: Permissions| perms.map(|x| x.contains(p)).unwrap_or(false);
    let has_role = |role: Option<u64>| match (role, component.member.as_ref()) {
        (Some(r), Some(m)) => m.roles.iter().any(|x| x.get() == r),
        _ => false,
    };
    (
        has(Permissions::ADMINISTRATOR),
        has(Permissions::MODERATE_MEMBERS),
        has(Permissions::MANAGE_MESSAGES),
        has_role(mod_role_id),
        has_role(admin_role_id),
    )
}
