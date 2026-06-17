//! Mode VOTE : alternative au mode review 1-clic (active par `vote_enabled`).
//!
//! Flux :
//!   1. Detection -> `post_vote_card` cree la review en API avec une echeance
//!      (`voting_deadline`, statut 'voting') et poste une carte avec des
//!      boutons de vote (Warn/Delete/Mute/Ban/Ignorer). custom_id
//!      `amv:<char>:<review_id>`.
//!   2. Chaque moderateur vote (`handle_vote_button`) -> POST /vote, la carte
//!      affiche le decompte a jour.
//!   3. A l'echeance, le worker appelle /decide -> event Redis
//!      `automod_review_decided` -> `handle_decided_event` edite la carte
//!      (verdict) et revele le bouton admin `amf:<review_id>`.
//!   4. L'admin clique (`handle_finalize_button`) -> POST /resolve
//!      (source=discord) + execution de la sanction Discord. L'admin
//!      confirme meme un refus (verdict 'ignore' = clore sans sanction).

use std::sync::Arc;

use serenity::model::channel::Message;
use serenity::prelude::*;
use tracing::{error, info, warn};

use crate::shared::api_client::BaseApiClient;
use crate::shared::heartbeat::ApiClientKey;

use super::api_client::Action;
use super::detectors;

pub(super) const VOTE_PREFIX: &str = "amv:";
pub(super) const FINALIZE_PREFIX: &str = "amf:";
/// Bouton "Ouvrir une discussion" -> cree un salon textuel prive (membre + modos).
pub(super) const DISCUSSION_PREFIX: &str = "amdisc:";
/// Bouton "Clore (ignorer)" -> clot immediatement le dossier (tout moderateur).
pub(super) const CLOSE_PREFIX: &str = "amclose:";
/// Bouton "Rouvrir le dossier" -> repasse en vote (tout moderateur).
pub(super) const REOPEN_PREFIX: &str = "amreopen:";

fn action_char(action: &Action) -> char {
    match action {
        Action::Warn => 'w',
        Action::Delete => 'd',
        Action::Mute => 'm',
        Action::Ban => 'b',
        Action::None => 'i',
    }
}

fn char_to_str(c: char) -> &'static str {
    match c {
        'p' => "prevention",
        'w' => "warn",
        'd' => "delete",
        'm' => "mute",
        'b' => "ban",
        _ => "ignore",
    }
}

fn action_label(s: &str) -> &'static str {
    match s {
        "prevention" => "Prevention",
        "warn" => "Avertissement",
        "delete" => "Suppression",
        "mute" => "Mute",
        "ban" => "Bannissement",
        _ => "Ignorer",
    }
}

/// Cree la review en mode vote et poste la carte avec les boutons de vote.
#[allow(clippy::too_many_arguments)]
pub(super) async fn post_vote_card(
    ctx: &Context,
    msg: &Message,
    suggested_action: &Action,
    reason: &str,
    score: f64,
    flags: &detectors::DetectionFlags,
    review_channel_id: u64,
    deadline_hours: i64,
    context_before: u8,
    thread_enabled: bool,
    aggregate: bool,
    discussion_enabled: bool,
    detail_url: Option<String>,
    auto_note: Option<String>,
) {
    if matches!(suggested_action, Action::None) {
        return;
    }
    let guild_id = msg.guild_id.map(|g| g.to_string()).unwrap_or_default();
    let channel_id = msg.channel_id.to_string();
    let message_id = msg.id.to_string();
    let user_id = msg.author.id.to_string();
    let content_preview = super::review::sanitize_embed_content(&msg.content, 500);

    let api = {
        let data = ctx.data.read().await;
        match data.get::<ApiClientKey>() {
            Some(a) => a.clone(),
            None => return,
        }
    };

    // 1. Creer la review en mode vote (avec echeance) pour obtenir son id.
    //    Si `aggregate`, l'API peut fusionner l'incident dans une carte
    //    'voting' ouverte du meme utilisateur -> on edite alors la carte
    //    existante au lieu d'en poster une nouvelle.
    let deadline = chrono::Utc::now() + chrono::Duration::hours(deadline_hours.clamp(1, 720));
    let suggested_str = char_to_str(action_char(suggested_action));
    let body = serde_json::json!({
        "guild_id": guild_id,
        "channel_id": channel_id,
        "message_id": message_id,
        "user_id": user_id,
        "user_name": msg.author.name,
        "content_preview": content_preview,
        "suggested_action": suggested_str,
        "score": score,
        "reason": reason,
        "flags": {
            "spam": flags.spam, "insult": flags.insult,
            "link": flags.link, "phishing": flags.phishing,
        },
        "voting_deadline": deadline.to_rfc3339(),
        "aggregate": aggregate,
    });

    let resp = match api.post_json::<_, ReviewResp>("/api/automod/reviews", &body).await {
        Ok(r) => r,
        Err(e) => {
            warn!(error = %e, "Echec creation review vote (sync degrade)");
            return;
        }
    };
    let review_id = resp.id.clone();

    // Cas agregation : l'incident a ete fusionne -> on edite la carte existante.
    // Si le message de cette carte a disparu (supprime), on ne `return` pas :
    // on retombe sur le posting normal ci-dessous pour reposter une carte neuve.
    if resp.merged && edit_aggregated_card(ctx, &api, &resp).await {
        return;
    }

    // 2. Recuperer le contexte (N messages avant) pour aider les moderateurs.
    let context = fetch_context_before(ctx, msg, context_before).await;

    // 3. Construire la carte. En mode agregation, layout enrichi (incidents).
    let mut embed = if aggregate {
        aggregated_vote_embed(&resp, &[])
    } else {
        vote_embed(
            &user_id, &msg.author.name, &channel_id, score, &content_preview, reason, flags,
            suggested_str, &deadline, &[],
        )
    };
    if !context.is_empty() {
        embed = embed.field("Contexte (messages precedents)", context, false);
    }
    // Action automatique deja appliquee (raid / phishing / pub / gros flood).
    if let Some(note) = &auto_note {
        embed = embed.field("🚨 Action automatique appliquee", note, false);
    }
    // 2e section : antecedents de moderation du membre (avec dates).
    if let Some(hist) = render_history_totals(ctx, &guild_id, &user_id).await {
        embed = embed.field("📋 Antecedents du membre", hist, false);
    }

    // Bouton lien : clic -> saute directement sur le message dans le salon.
    let msg_url = format!(
        "https://discord.com/channels/{}/{}/{}",
        guild_id, channel_id, message_id
    );
    let link_row = secondary_row(&msg_url, &review_id, discussion_enabled, detail_url.as_deref());

    let builder = serenity::builder::CreateMessage::new()
        .embed(embed)
        .components({
            let mut rows = vote_buttons(&review_id);
            rows.push(link_row);
            rows
        });

    let posted = match serenity::model::id::ChannelId::new(review_channel_id)
        .send_message(&ctx.http, builder)
        .await
    {
        Ok(p) => p,
        Err(e) => {
            error!(error = %e, review_channel = review_channel_id, "Echec envoi carte de vote");
            return;
        }
    };

    // 3. Enregistrer le mapping pour le sync (web + event decided).
    if let Ok(uuid) = uuid::Uuid::parse_str(&review_id) {
        crate::sync::register_action_message(
            &api,
            uuid,
            crate::sync::kinds::AUTOMOD_REVIEW,
            &guild_id,
            &posted.channel_id.to_string(),
            &posted.id.to_string(),
        )
        .await;
    }
    // Fil de discussion attache a la carte (debat des moderateurs).
    if thread_enabled {
        let thread_name = format!("Vote — {}", msg.author.name);
        let thread_name: String = thread_name.chars().take(90).collect();
        if let Err(e) = posted
            .channel_id
            .create_thread_from_message(
                &ctx.http,
                posted.id,
                serenity::builder::CreateThread::new(thread_name)
                    .auto_archive_duration(serenity::model::channel::AutoArchiveDuration::ThreeDays),
            )
            .await
        {
            warn!(error = %e, "Echec creation fil de discussion sur la carte de vote (permission CREATE_PUBLIC_THREADS ?)");
        }
    }

    info!(review_id, "Carte de vote automod postee");
}

/// Variante manuelle : une carte de vote creee par un moderateur via la
/// commande `/card` (et non par la detection automod). Difference cle : on
/// affiche le contexte AVANT **et** APRES le message pour donner le contexte
/// complet de l'echange. Reutilise le meme flux de review/vote/finalisation
/// que la carte automatique (memes boutons `amv:`/`amf:`, meme review en base).
#[allow(clippy::too_many_arguments)]
pub(super) async fn post_manual_vote_card(
    ctx: &Context,
    msg: &Message,
    suggested_action: &Action,
    reason: &str,
    review_channel_id: u64,
    deadline_hours: i64,
    context_count: u8,
    thread_enabled: bool,
    moderator_name: &str,
    discussion_enabled: bool,
    aggregate: bool,
    detail_url: Option<String>,
) {
    if matches!(suggested_action, Action::None) {
        return;
    }
    let guild_id = msg.guild_id.map(|g| g.to_string()).unwrap_or_default();
    let channel_id = msg.channel_id.to_string();
    let message_id = msg.id.to_string();
    let user_id = msg.author.id.to_string();
    let content_preview = super::review::sanitize_embed_content(&msg.content, 500);

    let api = {
        let data = ctx.data.read().await;
        match data.get::<ApiClientKey>() {
            Some(a) => a.clone(),
            None => return,
        }
    };

    // 1. Creer la review en mode vote (memes champs que la carte automod ;
    // score 0 et flags vides car signalement humain, pas IA).
    let deadline = chrono::Utc::now() + chrono::Duration::hours(deadline_hours.clamp(1, 720));
    let suggested_str = char_to_str(action_char(suggested_action));
    let body = serde_json::json!({
        "guild_id": guild_id,
        "channel_id": channel_id,
        "message_id": message_id,
        "user_id": user_id,
        "user_name": msg.author.name,
        "content_preview": content_preview,
        "suggested_action": suggested_str,
        "score": 0.0,
        "reason": reason,
        "flags": { "spam": false, "insult": false, "link": false, "phishing": false },
        "voting_deadline": deadline.to_rfc3339(),
        "aggregate": aggregate,
    });

    let resp = match api.post_json::<_, ReviewResp>("/api/automod/reviews", &body).await {
        Ok(r) => r,
        Err(e) => {
            warn!(error = %e, "Echec creation review (carte manuelle)");
            return;
        }
    };
    let review_id = resp.id.clone();

    // Agregation : si l'incident a ete fusionne, on edite la carte existante.
    // Si son message a disparu (supprime), on retombe sur le posting normal
    // ci-dessous pour reposter une carte neuve (mapping upserte).
    if resp.merged && edit_aggregated_card(ctx, &api, &resp).await {
        return;
    }

    // 2. Contexte avant ET apres le message cible.
    let before = fetch_context_before(ctx, msg, context_count).await;
    let after = fetch_context_after(ctx, msg, context_count).await;

    // 3. Construire la carte (embed dedie : pas de labels "IA").
    let mut embed = serenity::builder::CreateEmbed::new()
        .title("Signalement manuel -- VOTE des moderateurs")
        .color(0x5865f2)
        .field("Utilisateur", format!("<@{}> (`{}`)", user_id, msg.author.name), true)
        .field("Salon", format!("<#{}>", channel_id), true)
        .field("Signale par", format!("`{}`", moderator_name), true)
        .field("Message signale", format!("```{}```", content_preview), false)
        .field("Raison", reason, false)
        .field("Action suggeree", action_label(suggested_str), true)
        .field("Cloture", format!("<t:{}:R>", deadline.timestamp()), true);
    if !before.is_empty() {
        embed = embed.field("Contexte (avant)", before, false);
    }
    if !after.is_empty() {
        embed = embed.field("Contexte (apres)", after, false);
    }
    if let Some(hist) = render_history_totals(ctx, &guild_id, &user_id).await {
        embed = embed.field("📋 Antecedents du membre", hist, false);
    }
    embed = embed
        .field(VOTES_FIELD, render_votes(&[]), false)
        .footer(serenity::builder::CreateEmbedFooter::new(
            "Votez la sanction. A l'echeance, un admin finalise.",
        ))
        .timestamp(serenity::model::Timestamp::now());

    let msg_url = format!(
        "https://discord.com/channels/{}/{}/{}",
        guild_id, channel_id, message_id
    );
    let link_row = secondary_row(&msg_url, &review_id, discussion_enabled, detail_url.as_deref());

    let builder = serenity::builder::CreateMessage::new()
        .embed(embed)
        .components({
            let mut rows = vote_buttons(&review_id);
            rows.push(link_row);
            rows
        });

    let posted = match serenity::model::id::ChannelId::new(review_channel_id)
        .send_message(&ctx.http, builder)
        .await
    {
        Ok(p) => p,
        Err(e) => {
            error!(error = %e, review_channel = review_channel_id, "Echec envoi carte manuelle");
            return;
        }
    };

    // 4. Mapping pour le sync (web + event decided), identique a l'automod.
    if let Ok(uuid) = uuid::Uuid::parse_str(&review_id) {
        crate::sync::register_action_message(
            &api,
            uuid,
            crate::sync::kinds::AUTOMOD_REVIEW,
            &guild_id,
            &posted.channel_id.to_string(),
            &posted.id.to_string(),
        )
        .await;
    }
    if thread_enabled {
        let thread_name = format!("Vote — {}", msg.author.name);
        let thread_name: String = thread_name.chars().take(90).collect();
        if let Err(e) = posted
            .channel_id
            .create_thread_from_message(
                &ctx.http,
                posted.id,
                serenity::builder::CreateThread::new(thread_name)
                    .auto_archive_duration(serenity::model::channel::AutoArchiveDuration::ThreeDays),
            )
            .await
        {
            warn!(error = %e, "Echec creation fil sur la carte manuelle");
        }
    }

    info!(review_id, "Carte de vote manuelle postee");
}

/// Recupere jusqu'a `n` messages precedant le message signale et les rend
/// en bloc chronologique (du plus ancien au plus recent). Tronque pour
/// respecter la limite d'un field embed (1024 caracteres).
async fn fetch_context_before(ctx: &Context, msg: &Message, n: u8) -> String {
    if n == 0 {
        return String::new();
    }
    let limit = n.min(25);
    let before = match msg
        .channel_id
        .messages(
            &ctx.http,
            serenity::builder::GetMessages::new().before(msg.id).limit(limit),
        )
        .await
    {
        Ok(m) => m,
        Err(e) => {
            warn!(error = %e, "Echec recuperation contexte (messages avant)");
            return String::new();
        }
    };
    // L'API renvoie du plus recent au plus ancien -> on inverse.
    let mut lines: Vec<String> = Vec::new();
    let mut total = 0usize;
    for m in before.iter().rev() {
        let content = super::review::sanitize_embed_content(&m.content, 120);
        let content = if content.trim().is_empty() {
            "*(pièce jointe / embed)*".to_string()
        } else {
            content
        };
        let line = format!("**{}** : {}", m.author.name, content);
        // Limite field embed = 1024 ; on garde une marge.
        if total + line.len() + 1 > 1000 {
            lines.push("…".to_string());
            break;
        }
        total += line.len() + 1;
        lines.push(line);
    }
    lines.join("\n")
}

/// Comme `fetch_context_before` mais pour les messages POSTERIEURS au message
/// signale (utile pour la carte manuelle qui montre tout l'echange).
async fn fetch_context_after(ctx: &Context, msg: &Message, n: u8) -> String {
    if n == 0 {
        return String::new();
    }
    let limit = n.min(25);
    let after = match msg
        .channel_id
        .messages(
            &ctx.http,
            serenity::builder::GetMessages::new().after(msg.id).limit(limit),
        )
        .await
    {
        Ok(m) => m,
        Err(e) => {
            warn!(error = %e, "Echec recuperation contexte (messages apres)");
            return String::new();
        }
    };
    // L'API renvoie du plus recent au plus ancien -> on inverse pour l'ordre
    // chronologique (du plus proche du message cible au plus recent).
    let mut lines: Vec<String> = Vec::new();
    let mut total = 0usize;
    for m in after.iter().rev() {
        let content = super::review::sanitize_embed_content(&m.content, 120);
        let content = if content.trim().is_empty() {
            "*(pièce jointe / embed)*".to_string()
        } else {
            content
        };
        let line = format!("**{}** : {}", m.author.name, content);
        if total + line.len() + 1 > 1000 {
            lines.push("…".to_string());
            break;
        }
        total += line.len() + 1;
        lines.push(line);
    }
    lines.join("\n")
}

/// Rangees de boutons de vote (6 actions -> 2 rangees car Discord limite a 5
/// boutons par rangee). Ordre de severite : Prevention < Warn < Delete < Mute < Ban.
fn vote_buttons(review_id: &str) -> Vec<serenity::builder::CreateActionRow> {
    use serenity::all::ButtonStyle;
    use serenity::builder::{CreateActionRow, CreateButton};
    vec![
        CreateActionRow::Buttons(vec![
            CreateButton::new(format!("{VOTE_PREFIX}p:{review_id}")).label("Prevention").style(ButtonStyle::Success),
            CreateButton::new(format!("{VOTE_PREFIX}w:{review_id}")).label("Warn").style(ButtonStyle::Secondary),
            CreateButton::new(format!("{VOTE_PREFIX}d:{review_id}")).label("Delete").style(ButtonStyle::Secondary),
            CreateButton::new(format!("{VOTE_PREFIX}m:{review_id}")).label("Mute").style(ButtonStyle::Primary),
            CreateButton::new(format!("{VOTE_PREFIX}b:{review_id}")).label("Ban").style(ButtonStyle::Danger),
        ]),
        CreateActionRow::Buttons(vec![
            CreateButton::new(format!("{VOTE_PREFIX}i:{review_id}")).label("Ignorer (vote)").style(ButtonStyle::Secondary),
            CreateButton::new(format!("{CLOSE_PREFIX}{review_id}")).label("🚫 Clore (ignorer)").style(ButtonStyle::Danger),
        ]),
    ]
}

/// Rangee avec uniquement le bouton "Rouvrir le dossier" (carte close).
fn reopen_row(review_id: &str) -> serenity::builder::CreateActionRow {
    use serenity::all::ButtonStyle;
    use serenity::builder::{CreateActionRow, CreateButton};
    CreateActionRow::Buttons(vec![
        CreateButton::new(format!("{REOPEN_PREFIX}{review_id}"))
            .label("♻️ Rouvrir le dossier")
            .style(ButtonStyle::Secondary),
    ])
}

/// Antecedents du membre en TOTAUX (carte resumee). Le detail date est dans le
/// dashboard web. `None` si l'API moderation est indisponible.
pub(super) async fn render_history_totals(
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
pub(super) fn build_detail_url(
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

fn secondary_row(
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
fn vote_embed(
    user_id: &str,
    user_name: &str,
    channel_id: &str,
    score: f64,
    content_preview: &str,
    reason: &str,
    flags: &detectors::DetectionFlags,
    suggested: &str,
    deadline: &chrono::DateTime<chrono::Utc>,
    votes: &[VoteDto],
) -> serenity::builder::CreateEmbed {
    let mut flag_parts = Vec::new();
    if flags.spam { flag_parts.push("Spam"); }
    if flags.insult { flag_parts.push("Insulte"); }
    if flags.link { flag_parts.push("Lien"); }
    if flags.phishing { flag_parts.push("Phishing"); }
    let flags_str = if flag_parts.is_empty() { "Aucun".to_string() } else { flag_parts.join(", ") };

    serenity::builder::CreateEmbed::new()
        .title("AutoMod -- VOTE des moderateurs")
        .color(0x5865f2)
        .field("Utilisateur", format!("<@{}> (`{}`)", user_id, user_name), true)
        .field("Salon", format!("<#{}>", channel_id), true)
        .field("Score IA", format!("{:.2}", score), true)
        .field("Message original", format!("```{}```", content_preview), false)
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
const VOTES_FIELD: &str = "Votes";

/// Rendu nominatif des votes, groupes par sanction :
/// `Avertissement (2) : Alice, Bob`.
fn render_votes(votes: &[VoteDto]) -> String {
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
        lines.push(format!("**{}** ({}) : {}", action_label(a), voters.len(), voters.join(", ")));
    }
    if lines.is_empty() {
        "_Aucun vote pour l'instant._".to_string()
    } else {
        lines.join("\n")
    }
}

#[derive(serde::Deserialize)]
struct VoteDto {
    #[allow(dead_code)]
    voter_id: String,
    voter_name: String,
    vote_action: String,
}

/// Reponse de POST /api/automod/reviews — champs utiles a la carte agregee.
#[derive(serde::Deserialize, Default)]
struct ReviewResp {
    id: String,
    #[serde(default)]
    merged: bool,
    #[serde(default)]
    guild_id: String,
    #[serde(default)]
    user_id: String,
    #[serde(default)]
    channel_id: String,
    #[serde(default)]
    content_preview: String,
    #[serde(default)]
    reason: String,
    #[serde(default)]
    suggested_action: String,
    #[serde(default)]
    score: f64,
    #[serde(default)]
    cumulative_score: f64,
    #[serde(default)]
    incident_count: i32,
    #[serde(default)]
    voting_deadline: Option<String>,
    #[serde(default)]
    incidents: serde_json::Value,
}

/// Construit l'embed d'une carte de vote AGREGEE (plusieurs incidents pour un
/// meme utilisateur). Affiche score max ET score cumule + nb d'incidents.
fn aggregated_vote_embed(resp: &ReviewResp, votes: &[VoteDto]) -> serenity::builder::CreateEmbed {
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
        .field("Score cumule", format!("{:.2}", resp.cumulative_score), true)
        .field("Action suggeree", action_label(&resp.suggested_action), true);
    if let Some(ts) = deadline_ts {
        embed = embed.field("Cloture", format!("<t:{}:R>", ts), true);
    }
    embed = embed
        .field("Dernier message", format!("```{}```", resp.content_preview), false)
        .field("Raison", if resp.reason.is_empty() { "—" } else { resp.reason.as_str() }, false);
    // Le detail complet des incidents est dans le dashboard web (bouton lien).
    embed = embed
        .field(VOTES_FIELD, render_votes(votes), false)
        .footer(serenity::builder::CreateEmbedFooter::new(
            "Alertes regroupees. Votez la sanction ; a l'echeance, un admin finalise.",
        ))
        .timestamp(serenity::model::Timestamp::now());
    embed
}

/// Archive le salon de discussion lie a une review finalisee : renomme en
/// "clos-…" et retire le droit d'ecrire au membre concerne (les moderateurs
/// gardent l'acces en lecture pour la trace). No-op si aucun salon.
async fn archive_discussion_channel(ctx: &Context, api: &Arc<BaseApiClient>, review_id: &str, target_user_id: &str) {
    use serenity::all::{ChannelId, Permissions, UserId};
    use serenity::model::channel::{Channel, PermissionOverwrite, PermissionOverwriteType};

    #[derive(serde::Deserialize)]
    struct DiscussionResp { channel_id: String }
    let Ok(Some(disc)) = api
        .get_json::<Option<DiscussionResp>>(&format!("/api/automod/reviews/{review_id}/discussion"))
        .await
    else {
        return;
    };
    let Ok(cid) = disc.channel_id.parse::<u64>() else { return };
    let channel = ChannelId::new(cid);

    // Renomme pour marquer l'affaire close.
    if let Ok(Channel::Guild(gc)) = channel.to_channel(&ctx.http).await {
        if !gc.name.starts_with("clos-") {
            let new_name: String = format!("clos-{}", gc.name).chars().take(95).collect();
            let _ = channel
                .edit(&ctx.http, serenity::builder::EditChannel::new().name(new_name))
                .await;
        }
    }
    // Verrouille l'ecriture pour le membre concerne.
    if let Ok(uid) = target_user_id.parse::<u64>() {
        let _ = channel
            .create_permission(
                &ctx.http,
                PermissionOverwrite {
                    allow: Permissions::empty(),
                    deny: Permissions::SEND_MESSAGES,
                    kind: PermissionOverwriteType::Member(UserId::new(uid)),
                },
            )
            .await;
    }
    info!(review_id, channel = %channel, "Salon de discussion archive (review finalisee)");
}

/// Edite la carte existante d'une review agregee : recharge le mapping Discord
/// + les votes, puis remplace l'embed (les boutons de vote sont conserves).
///
/// Retourne `true` si la carte a ete editee (ou si l'echec est transitoire et
/// ne justifie pas de recreer la carte), `false` si le message de la carte a
/// disparu (supprime cote Discord) — dans ce cas l'appelant doit reposter une
/// carte neuve (le mapping `discord_action_messages` etant upserte, le nouveau
/// message_id remplace l'ancien).
async fn edit_aggregated_card(ctx: &Context, api: &Arc<BaseApiClient>, resp: &ReviewResp) -> bool {
    use serenity::all::{ChannelId, MessageId};

    #[derive(serde::Deserialize)]
    struct Mapping { kind: String, channel_id: String, message_id: String }
    let mappings: Vec<Mapping> = match api.get_json(&format!("/api/discord-messages/{}", resp.id)).await {
        Ok(m) => m,
        Err(e) => { warn!(error = %e, review_id = %resp.id, "Echec fetch mapping (agregation)"); return true; }
    };
    // Pas de mapping connu -> on ne peut pas editer : il faut (re)creer la carte.
    let Some(mapping) = mappings.into_iter().find(|m| m.kind == "automod_review") else { return false };
    let (Ok(cid), Ok(mid)) = (mapping.channel_id.parse::<u64>(), mapping.message_id.parse::<u64>()) else { return false };
    let channel_id = ChannelId::new(cid);
    let msg_id = MessageId::new(mid);

    // Recharge les votes pour les conserver dans l'embed reconstruit.
    let votes: Vec<VoteDto> = api
        .get_json(&format!("/api/automod/reviews/{}/votes", resp.id))
        .await
        .unwrap_or_default();

    let mut embed = aggregated_vote_embed(resp, &votes);
    if let Some(hist) = render_history_totals(ctx, &resp.guild_id, &resp.user_id).await {
        embed = embed.field("📋 Antecedents du membre", hist, false);
    }
    // On n'edite que l'embed : les composants (boutons de vote + lien) restent.
    match channel_id
        .edit_message(&ctx.http, msg_id, serenity::builder::EditMessage::new().embed(embed))
        .await
    {
        Ok(_) => {
            info!(review_id = %resp.id, incidents = resp.incident_count, "Carte agregee mise a jour");
            true
        }
        Err(e) => {
            // Message (ou salon) supprime cote Discord -> il faut recreer la carte.
            // Pour les autres erreurs (rate limit, reseau...) on NE recree PAS,
            // afin d'eviter les doublons : on retourne true (rien a recreer).
            let s = e.to_string();
            if s.contains("Unknown Message") || s.contains("Unknown Channel") || s.contains("10008") || s.contains("10003") {
                warn!(review_id = %resp.id, "Carte agregee introuvable (message supprime) -> recreation");
                false
            } else {
                warn!(error = %e, review_id = %resp.id, "Echec edition carte agregee (erreur transitoire, pas de recreation)");
                true
            }
        }
    }
}

/// Collecte les FAITS Discord du demandeur (permissions + appartenance aux
/// roles configures). La DECISION d'autorisation (is_moderator / can_finalize)
/// est prise cote core (full hexa). Retourne
/// `(is_admin, has_moderate_members, has_manage_messages, has_mod_role, has_admin_role)`.
fn moderator_facts(
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

/// Edite une reponse deja deferee (ephemere). A utiliser apres un Defer.
async fn edit_ephemeral(ctx: &Context, component: &serenity::model::application::ComponentInteraction, text: &str) {
    let _ = component
        .edit_response(
            &ctx.http,
            serenity::builder::EditInteractionResponse::new().content(text),
        )
        .await;
}

async fn reply_ephemeral(ctx: &Context, component: &serenity::model::application::ComponentInteraction, text: &str) {
    let _ = component
        .create_response(
            &ctx.http,
            serenity::builder::CreateInteractionResponse::Message(
                serenity::builder::CreateInteractionResponseMessage::new()
                    .content(text)
                    .ephemeral(true),
            ),
        )
        .await;
}

/// Handler du bouton de vote (`amv:<char>:<review_id>`).
pub(super) async fn handle_vote_button(ctx: &Context, component: &serenity::model::application::ComponentInteraction) {
    let rest = match component.data.custom_id.strip_prefix(VOTE_PREFIX) {
        Some(r) => r,
        None => return,
    };
    let (char_part, review_id) = match rest.split_once(':') {
        Some((c, id)) => (c, id),
        None => return,
    };
    let vote_action = char_to_str(char_part.chars().next().unwrap_or('i'));

    let guild_id = component.guild_id.map(|g| g.to_string()).unwrap_or_default();
    let api = {
        let data = ctx.data.read().await;
        match data.get::<ApiClientKey>() { Some(a) => a.clone(), None => return }
    };
    let config = api.get_guild_config_for(&guild_id, super::MODULE_BOT_NAME).await.unwrap_or_default();
    let mod_role_id = config.get("vote_mod_role_id").and_then(|v| v.parse::<u64>().ok()).filter(|x| *x > 0);

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
            reply_ephemeral(ctx, component, "Vote impossible (non autorise ou vote clos).").await;
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
            rebuilt = rebuilt.footer(serenity::builder::CreateEmbedFooter::new(footer.text.clone()));
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
        reply_ephemeral(ctx, component, &format!("Vote enregistre : {}.", action_label(vote_action))).await;
    }
    info!(review_id, voter = %component.user.name, vote = vote_action, "Vote automod enregistre");
}

/// Event Redis `automod_review_decided` : edite la carte (verdict) et
/// ajoute le bouton admin de finalisation.
pub(super) async fn handle_decided_event(ctx: &Context, payload: &str) {
    use serenity::all::{ChannelId, GetMessages, MessageId};
    let event: serde_json::Value = match serde_json::from_str(payload) { Ok(v) => v, Err(_) => return };
    if event.get("event").and_then(|e| e.as_str()) != Some("automod_review_decided") {
        return;
    }
    let data = match event.get("data") { Some(d) => d, None => return };
    let action_id = data.get("action_id").and_then(|v| v.as_str()).unwrap_or("");
    if action_id.is_empty() { return; }
    let decided_action = data.get("decided_action").and_then(|v| v.as_str()).unwrap_or("ignore");
    let quorum_met = data.get("quorum_met").and_then(|v| v.as_bool()).unwrap_or(false);
    let total_votes = data.get("total_votes").and_then(|v| v.as_u64()).unwrap_or(0);

    let api = {
        let d = ctx.data.read().await;
        match d.get::<ApiClientKey>() { Some(a) => a.clone(), None => return }
    };

    #[derive(serde::Deserialize)]
    struct Mapping { kind: String, channel_id: String, message_id: String }
    let mappings: Vec<Mapping> = match api.get_json(&format!("/api/discord-messages/{action_id}")).await {
        Ok(l) => l,
        Err(e) => { warn!(error = %e, action_id, "Echec fetch mapping (decided)"); return; }
    };
    let mapping = match mappings.into_iter().find(|m| m.kind == "automod_review") { Some(m) => m, None => return };
    let channel_id = match mapping.channel_id.parse::<u64>() { Ok(v) => ChannelId::new(v), Err(_) => return };
    let msg_id = match mapping.message_id.parse::<u64>() { Ok(v) => MessageId::new(v), Err(_) => return };

    let verdict = if !quorum_met {
        format!("Quorum non atteint ({total_votes} votes) -> aucune sanction. Un admin doit clore.")
    } else {
        format!("Verdict : **{}** ({total_votes} votes). En attente de finalisation par un admin.", action_label(decided_action))
    };

    let finalize_btn = serenity::builder::CreateButton::new(format!("{FINALIZE_PREFIX}{action_id}"))
        .label(format!("Finaliser ({})", action_label(decided_action)))
        .style(serenity::all::ButtonStyle::Success);
    let close_btn = serenity::builder::CreateButton::new(format!("{CLOSE_PREFIX}{action_id}"))
        .label("🚫 Clore (ignorer)")
        .style(serenity::all::ButtonStyle::Danger);
    let row = serenity::builder::CreateActionRow::Buttons(vec![finalize_btn, close_btn]);

    if let Ok(messages) = channel_id.messages(&ctx.http, GetMessages::new().limit(1).around(msg_id)).await {
        if let Some(original) = messages.into_iter().find(|m| m.id == msg_id) {
            if let Some(existing) = original.embeds.first() {
                let new_embed = serenity::builder::CreateEmbed::from(existing.clone())
                    .color(0xf1c40f)
                    .field("Vote clos", verdict, false)
                    .timestamp(serenity::model::Timestamp::now());
                let _ = channel_id
                    .edit_message(
                        &ctx.http,
                        msg_id,
                        serenity::builder::EditMessage::new().embed(new_embed).components(vec![row]),
                    )
                    .await;
            }
        }
    }
    info!(action_id, decided_action, quorum_met, "Carte vote editee (decided)");
}

/// Handler du bouton admin de finalisation (`amf:<review_id>`).
pub(super) async fn handle_finalize_button(ctx: &Context, component: &serenity::model::application::ComponentInteraction) {
    let review_id = match component.data.custom_id.strip_prefix(FINALIZE_PREFIX) {
        Some(r) => r.to_string(),
        None => return,
    };
    let guild_id = component.guild_id.map(|g| g.to_string()).unwrap_or_default();
    let api = {
        let data = ctx.data.read().await;
        match data.get::<ApiClientKey>() { Some(a) => a.clone(), None => return }
    };
    let config = api.get_guild_config_for(&guild_id, super::MODULE_BOT_NAME).await.unwrap_or_default();
    let admin_role_id = config.get("vote_admin_role_id").and_then(|v| v.parse::<u64>().ok()).filter(|x| *x > 0);

    // Faits Discord -> la regle can_finalize_review est appliquee cote core
    // (full hexa). Un non-admin recevra une erreur 403 a l'etape /resolve.
    let facts = moderator_facts(component, None, admin_role_id);

    // Recupere la review (verdict + cible) depuis l'API.
    #[derive(serde::Deserialize)]
    struct ReviewDto {
        channel_id: String,
        message_id: String,
        user_id: String,
        decided_action: Option<String>,
        status: String,
    }
    let review: ReviewDto = match api.get_json(&format!("/api/automod/reviews/{review_id}")).await {
        Ok(r) => r,
        Err(e) => {
            warn!(error = %e, review_id, "Echec fetch review (finalize)");
            reply_ephemeral(ctx, component, "Review introuvable.").await;
            return;
        }
    };
    if review.status != "decided" {
        reply_ephemeral(ctx, component, &format!("Cette review n'est pas finalisable (statut : {}).", review.status)).await;
        return;
    }
    let decided = review.decided_action.clone().unwrap_or_else(|| "ignore".to_string());

    // Persiste la resolution cote API (source discord) + faits du demandeur.
    let resolve_body = serde_json::json!({
        "applied_action": decided,
        "resolved_by_id": component.user.id.to_string(),
        "resolved_by_name": component.user.name,
        "source": "discord",
        "is_admin": facts.0,
        "has_admin_role": facts.4,
    });
    if let Err(e) = api
        .post_json::<_, serde_json::Value>(&format!("/api/automod/reviews/{review_id}/resolve"), &resolve_body)
        .await
    {
        warn!(error = %e, review_id, "Echec resolve finalize (non autorise ou deja finalise)");
        reply_ephemeral(ctx, component, "Finalisation impossible (reserve aux admins, ou deja finalise).").await;
        return;
    }

    // La sanction de membre est tracee cote API (dans la meme requete /resolve
    // ci-dessus) -> plus de 2e appel HTTP ici (evite la fenetre "resolu mais
    // non logge"). Le bot se contente d'executer l'action Discord.
    let mute_secs = BaseApiClient::config_u64(&config, "mute_duration_secs", super::DEFAULT_MUTE_DURATION_SECS);

    // Execute la sanction Discord (delete/mute/ban).
    apply_member_sanction(ctx, component.guild_id, &review.channel_id, &review.message_id, &review.user_id, &decided, mute_secs).await;

    // Notice membre (cohérence de ton avec les autres chemins) : on informe le
    // membre en DM de la sanction validée + droit d'appel. Best-effort.
    if matches!(decided.as_str(), "prevention" | "warn" | "mute" | "ban") {
        let appeal = BaseApiClient::config_bool(&config, "sanction_appeal_enabled", true);
        let mins = if decided == "mute" { Some(mute_secs / 60) } else { None };
        let embed = crate::shared::embeds::sanction_notice(
            &decided,
            "Décision validée par les modérateurs",
            mins,
            Some(&component.user.name),
            appeal,
        );
        if let Ok(uid) = review.user_id.parse::<u64>() {
            if let Ok(ch) = serenity::model::id::UserId::new(uid).create_dm_channel(&ctx.http).await {
                let _ = ch
                    .send_message(&ctx.http, serenity::builder::CreateMessage::new().embed(embed))
                    .await;
            }
        }
    }

    // Archive le salon de discussion lie (s'il existe) : l'affaire est close.
    archive_discussion_channel(ctx, &api, &review_id, &review.user_id).await;

    // Edite la carte : finalise.
    let finalized = serenity::builder::CreateEmbed::new()
        .title("AutoMod -- Vote finalise")
        .description(format!(
            "Sanction : **{}**\nFinalise par : **{}**\nCible : <@{}>",
            action_label(&decided), component.user.name, review.user_id
        ))
        .color(0x2ecc71)
        .footer(serenity::builder::CreateEmbedFooter::new("AutoMod Vote | Finalise par un admin"))
        .timestamp(serenity::model::Timestamp::now());
    let _ = component
        .create_response(
            &ctx.http,
            serenity::builder::CreateInteractionResponse::UpdateMessage(
                serenity::builder::CreateInteractionResponseMessage::new()
                    .embed(finalized)
                    .components(vec![reopen_row(&review_id)]),
            ),
        )
        .await;
    info!(review_id, action = %decided, admin = %component.user.name, "Vote automod finalise");
}

/// Handler du bouton "Clore (ignorer)" (`amclose:<review_id>`).
/// Tout moderateur peut clore immediatement : statut -> ignored, aucune
/// sanction, carte mise a jour + bouton "Rouvrir le dossier".
pub(super) async fn handle_close_button(ctx: &Context, component: &serenity::model::application::ComponentInteraction) {
    let review_id = match component.data.custom_id.strip_prefix(CLOSE_PREFIX) {
        Some(r) => r.to_string(),
        None => return,
    };
    let guild_id = component.guild_id.map(|g| g.to_string()).unwrap_or_default();
    let api = {
        let data = ctx.data.read().await;
        match data.get::<ApiClientKey>() { Some(a) => a.clone(), None => return }
    };
    let config = api.get_guild_config_for(&guild_id, super::MODULE_BOT_NAME).await.unwrap_or_default();
    let mod_role_id = config.get("vote_mod_role_id").and_then(|v| v.parse::<u64>().ok()).filter(|x| *x > 0);
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
        .post_json::<_, serde_json::Value>(&format!("/api/automod/reviews/{review_id}/ignore"), &body)
        .await
    {
        warn!(error = %e, review_id, "Echec clore (ignorer) : non autorise ou deja clos");
        reply_ephemeral(ctx, component, "Cloture impossible (reserve aux moderateurs, ou deja clos).").await;
        return;
    }

    // Archive le salon de discussion lie (s'il existe) : l'affaire est close.
    // (user_id recupere best-effort via l'API pour verrouiller l'ecriture.)
    #[derive(serde::Deserialize)]
    struct U { user_id: String }
    if let Ok(r) = api.get_json::<U>(&format!("/api/automod/reviews/{review_id}")).await {
        archive_discussion_channel(ctx, &api, &review_id, &r.user_id).await;
    }

    let closed = serenity::builder::CreateEmbed::new()
        .title("AutoMod -- Dossier clos (ignore)")
        .description(format!(
            "Clos par **{}**. Aucune sanction appliquee.\nUn moderateur peut rouvrir le dossier si besoin.",
            component.user.name
        ))
        .color(0x95a5a6)
        .footer(serenity::builder::CreateEmbedFooter::new("AutoMod | Dossier ignore"))
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
pub(super) async fn handle_reopen_button(ctx: &Context, component: &serenity::model::application::ComponentInteraction) {
    let review_id = match component.data.custom_id.strip_prefix(REOPEN_PREFIX) {
        Some(r) => r.to_string(),
        None => return,
    };
    let guild_id = component.guild_id.map(|g| g.to_string()).unwrap_or_default();
    let api = {
        let data = ctx.data.read().await;
        match data.get::<ApiClientKey>() { Some(a) => a.clone(), None => return }
    };
    let config = api.get_guild_config_for(&guild_id, super::MODULE_BOT_NAME).await.unwrap_or_default();
    let mod_role_id = config.get("vote_mod_role_id").and_then(|v| v.parse::<u64>().ok()).filter(|x| *x > 0);
    let deadline_hours = BaseApiClient::config_u64(&config, "vote_deadline_hours", 72) as i64;
    let discussion_enabled = BaseApiClient::config_bool(&config, "discussion_channel_enabled", false);
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
            reply_ephemeral(ctx, component, "Reouverture impossible (reserve aux moderateurs, ou deja ouvert).").await;
            return;
        }
    };

    let flags = detectors::DetectionFlags {
        spam: review.flags.get("spam").and_then(|v| v.as_bool()).unwrap_or(false),
        insult: review.flags.get("insult").and_then(|v| v.as_bool()).unwrap_or(false),
        link: review.flags.get("link").and_then(|v| v.as_bool()).unwrap_or(false),
        phishing: review.flags.get("phishing").and_then(|v| v.as_bool()).unwrap_or(false),
    };
    let deadline = review
        .voting_deadline
        .as_deref()
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|d| d.with_timezone(&chrono::Utc))
        .unwrap_or_else(|| chrono::Utc::now() + chrono::Duration::hours(deadline_hours));

    let mut embed = vote_embed(
        &review.user_id, &review.user_name, &review.channel_id, review.score,
        &review.content_preview, &review.reason, &flags, &review.suggested_action, &deadline, &[],
    );
    if let Some(hist) = render_history_totals(ctx, &review.guild_id, &review.user_id).await {
        embed = embed.field("📋 Antecedents du membre", hist, false);
    }
    embed = embed.field(
        "♻️ Dossier rouvert",
        format!("Rouvert par **{}** — nouveau vote en cours.", component.user.name),
        false,
    );

    let msg_url = format!(
        "https://discord.com/channels/{}/{}/{}",
        review.guild_id, review.channel_id, review.message_id
    );
    let link_row = secondary_row(&msg_url, &review_id, discussion_enabled, detail_url.as_deref());
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

/// Handler du bouton "Ouvrir une discussion" (`amdisc:<review_id>`).
/// Cree un salon textuel prive (membre concerne + role modo) sous la categorie
/// configuree, avec un message de contexte epingle ("ancrage").
pub(super) async fn handle_discussion_button(
    ctx: &Context,
    component: &serenity::model::application::ComponentInteraction,
) {
    use serenity::all::{ChannelId, Permissions, RoleId, UserId};
    use serenity::model::channel::{PermissionOverwrite, PermissionOverwriteType};

    let review_id = match component.data.custom_id.strip_prefix(DISCUSSION_PREFIX) {
        Some(r) => r.to_string(),
        None => return,
    };
    let guild_id = match component.guild_id {
        Some(g) => g,
        None => return,
    };

    // Defer ephemere : la creation de salon peut depasser les 3s d'ack.
    if component
        .create_response(
            &ctx.http,
            serenity::builder::CreateInteractionResponse::Defer(
                serenity::builder::CreateInteractionResponseMessage::new().ephemeral(true),
            ),
        )
        .await
        .is_err()
    {
        return;
    }

    let api = {
        let d = ctx.data.read().await;
        match d.get::<ApiClientKey>() { Some(a) => a.clone(), None => return }
    };
    let config = api
        .get_guild_config_for(&guild_id.to_string(), super::MODULE_BOT_NAME)
        .await
        .unwrap_or_default();

    if !BaseApiClient::config_bool(&config, "discussion_channel_enabled", false) {
        edit_ephemeral(ctx, component, "La creation de salon de discussion est desactivee.").await;
        return;
    }
    let mod_role_id = config.get("vote_mod_role_id").and_then(|v| v.parse::<u64>().ok()).filter(|x| *x > 0);

    // Reponse de l'API discussion (GET existant + POST open). La REGLE d'acces
    // est appliquee cote core (full hexa) ; le bot ne fait que relayer.
    #[derive(serde::Deserialize)]
    struct DiscussionResp {
        channel_id: String,
        #[serde(default)]
        created: bool,
    }
    // Idempotence : un salon existe deja ? on s'y refere sans rien creer.
    if let Ok(Some(existing)) = api
        .get_json::<Option<DiscussionResp>>(&format!("/api/automod/reviews/{review_id}/discussion"))
        .await
    {
        edit_ephemeral(ctx, component, &format!("Un salon de discussion existe deja : <#{}>", existing.channel_id)).await;
        return;
    }

    // Recupere la review (cible + contexte).
    #[derive(serde::Deserialize)]
    struct ReviewDto {
        guild_id: String,
        channel_id: String,
        message_id: String,
        user_id: String,
        user_name: String,
        suggested_action: Option<String>,
        reason: String,
        score: f64,
    }
    let review: ReviewDto = match api.get_json(&format!("/api/automod/reviews/{review_id}")).await {
        Ok(r) => r,
        Err(e) => {
            warn!(error = %e, review_id, "Echec fetch review (discussion)");
            edit_ephemeral(ctx, component, "Review introuvable.").await;
            return;
        }
    };
    let target_uid = match review.user_id.parse::<u64>() {
        Ok(v) => UserId::new(v),
        Err(_) => return,
    };

    // Overwrites : @everyone deny view ; cible + role modo + bot allow.
    let participate =
        Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES | Permissions::READ_MESSAGE_HISTORY;
    let mut overwrites = vec![
        PermissionOverwrite {
            allow: Permissions::empty(),
            deny: Permissions::VIEW_CHANNEL,
            kind: PermissionOverwriteType::Role(RoleId::new(guild_id.get())),
        },
        PermissionOverwrite {
            // On n'accorde QUE `participate` : accorder MANAGE_MESSAGES ici ferait
            // echouer toute la creation si le bot n'a pas exactement cette perm au
            // niveau serveur (Discord interdit d'accorder une perm qu'on n'a pas).
            // Le pin du message d'ancrage utilise les perms serveur du bot.
            allow: participate,
            deny: Permissions::empty(),
            kind: PermissionOverwriteType::Member(ctx.cache.current_user().id),
        },
        // Le moderateur qui ouvre la discussion a toujours acces, meme si aucun
        // role modo n'est configure (sinon le salon lui serait invisible).
        PermissionOverwrite {
            allow: participate,
            deny: Permissions::empty(),
            kind: PermissionOverwriteType::Member(component.user.id),
        },
    ];
    if let Some(role) = mod_role_id {
        overwrites.push(PermissionOverwrite {
            allow: participate,
            deny: Permissions::empty(),
            kind: PermissionOverwriteType::Role(RoleId::new(role)),
        });
    }

    // Nom de salon : "discussion-pseudo" assaini (alnum + tirets, sans doublons
    // de tirets ni tirets en bord ; repli sur l'id si le pseudo donne un nom vide).
    let mapped: String = format!("discussion-{}", review.user_name)
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let mut collapsed = mapped;
    while collapsed.contains("--") {
        collapsed = collapsed.replace("--", "-");
    }
    let trimmed = collapsed.trim_matches('-').to_string();
    let name: String = if trimmed.is_empty() {
        format!("discussion-{}", review.user_id)
    } else {
        trimmed.chars().take(95).collect()
    };

    let cat_id = config
        .get("discussion_category_id")
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|x| *x > 0);

    let build = |with_cat: bool| {
        let mut b = serenity::builder::CreateChannel::new(name.clone())
            .kind(serenity::model::channel::ChannelType::Text)
            .permissions(overwrites.clone());
        if with_cat {
            if let Some(c) = cat_id {
                b = b.category(ChannelId::new(c));
            }
        }
        b
    };

    // Cree le salon ; si echec AVEC categorie, on retente SANS (cause frequente :
    // categorie invalide/pleine). L'erreur Discord reelle est remontee a l'admin.
    let channel = match guild_id.create_channel(&ctx.http, build(true)).await {
        Ok(c) => c,
        Err(e1) if cat_id.is_some() => {
            warn!(error = %e1, "Echec creation salon discussion (avec categorie) -- retry sans categorie");
            match guild_id.create_channel(&ctx.http, build(false)).await {
                Ok(c) => c,
                Err(e2) => {
                    warn!(error = %e2, "Echec creation salon discussion (sans categorie)");
                    edit_ephemeral(ctx, component, &format!("Echec creation du salon : {e2}")).await;
                    return;
                }
            }
        }
        Err(e) => {
            warn!(error = %e, "Echec creation salon discussion");
            edit_ephemeral(ctx, component, &format!("Echec creation du salon : {e}")).await;
            return;
        }
    };

    // Donne l'acces au membre concerne APRES creation (best-effort) : s'il a
    // quitte/est banni, l'overwrite peut echouer sans bloquer la creation.
    if let Err(e) = channel
        .id
        .create_permission(
            &ctx.http,
            PermissionOverwrite {
                allow: participate,
                deny: Permissions::empty(),
                kind: PermissionOverwriteType::Member(target_uid),
            },
        )
        .await
    {
        warn!(error = %e, user = %target_uid, "guild discussion: acces membre cible non accorde (a-t-il quitte ?)");
    }

    // Enregistre le salon cote API : le domaine applique la regle d'acces
    // (can_open_discussion) sur les faits Discord du demandeur + idempotence.
    let perms = component.member.as_ref().and_then(|m| m.permissions);
    let has = |p: Permissions| perms.map(|x| x.contains(p)).unwrap_or(false);
    let has_mod_role = match (mod_role_id, component.member.as_ref()) {
        (Some(role), Some(m)) => m.roles.iter().any(|r| r.get() == role),
        _ => false,
    };
    let open_body = serde_json::json!({
        "guild_id": guild_id.to_string(),
        "channel_id": channel.id.to_string(),
        "opened_by_id": component.user.id.to_string(),
        "opened_by_name": component.user.name,
        "is_admin": has(Permissions::ADMINISTRATOR),
        "has_moderate_members": has(Permissions::MODERATE_MEMBERS),
        "has_manage_messages": has(Permissions::MANAGE_MESSAGES),
        "has_mod_role": has_mod_role,
    });
    let opened: DiscussionResp = match api
        .post_json(&format!("/api/automod/reviews/{review_id}/discussion"), &open_body)
        .await
    {
        Ok(r) => r,
        Err(e) => {
            // 403 (non autorise) ou autre erreur : on annule le salon cree.
            warn!(error = %e, review_id, "Refus/echec enregistrement discussion -> suppression du salon");
            let _ = channel.id.delete(&ctx.http).await;
            edit_ephemeral(ctx, component, "Discussion non autorisee ou erreur : salon annule.").await;
            return;
        }
    };
    if !opened.created {
        // Course : un salon a ete enregistre entre-temps -> on annule le notre.
        let _ = channel.id.delete(&ctx.http).await;
        edit_ephemeral(ctx, component, &format!("Un salon de discussion existe deja : <#{}>", opened.channel_id)).await;
        return;
    }

    // Message d'ancrage epingle (contexte de la moderation).
    let action = review.suggested_action.as_deref().unwrap_or("warn");
    let origin_url = format!(
        "https://discord.com/channels/{}/{}/{}",
        review.guild_id, review.channel_id, review.message_id
    );
    let anchor = serenity::builder::CreateEmbed::new()
        .title("Discussion de moderation")
        .color(0x5865f2)
        .field("Membre", format!("<@{}> (`{}`)", review.user_id, review.user_name), true)
        .field("Action envisagee", action_label(action), true)
        .field("Score", format!("{:.2}", review.score), true)
        .field("Raison", if review.reason.is_empty() { "—" } else { review.reason.as_str() }, false)
        .field("Message d'origine", format!("[Aller au message]({origin_url})"), false)
        .footer(serenity::builder::CreateEmbedFooter::new(
            "Salon ouvert pour echanger avant decision.",
        ))
        .timestamp(serenity::model::Timestamp::now());
    let ping = match mod_role_id {
        Some(role) => format!("<@{}> <@&{}>", review.user_id, role),
        None => format!("<@{}>", review.user_id),
    };
    if let Ok(posted) = channel
        .id
        .send_message(&ctx.http, serenity::builder::CreateMessage::new().content(ping).embed(anchor))
        .await
    {
        // "Ancrage" = epinglage du message de contexte en haut du salon.
        let _ = channel.id.pin(&ctx.http, posted.id).await;
    }

    edit_ephemeral(ctx, component, &format!("Salon de discussion cree : <#{}>", channel.id)).await;
    info!(review_id, channel = %channel.id, "Salon de discussion cree");
}

/// Enregistre une infraction warn via le module moderation (gRPC log_action),
/// de sorte que le warn issu d'un vote compte dans l'historique et l'escalade
/// au meme titre qu'un /warn manuel. L'admin qui finalise est le "moderateur".
/// Trace une sanction de membre (warn/mute/ban) dans le module moderation via
/// gRPC log_action, pour qu'elle compte dans l'historique et l'escalade au
/// meme titre qu'une commande manuelle. `duration` = duree du mute en secondes.
/// Partage par le vote (finalisation) et la review 1-clic.
#[allow(clippy::too_many_arguments)]
pub(super) async fn log_sanction_to_moderation(
    ctx: &Context,
    guild_id: &str,
    action_channel_id: &str,
    moderator_id: &str,
    moderator_name: &str,
    target_id: &str,
    target_name: &str,
    action_type: &str,
    reason: &str,
    duration: Option<u64>,
) {
    use crate::modules::moderation::api_client::ModerationAction;
    use crate::modules::moderation::ModerationApiKey;

    // Sanctions de membre tracees dans l'historique (prevention incluse, mais
    // elle ne compte pas dans l'escalade -- gere cote service log_action_with_strike).
    if !matches!(action_type, "prevention" | "warn" | "mute" | "ban") {
        return;
    }

    let mod_api = {
        let data = ctx.data.read().await;
        match data.get::<ModerationApiKey>() {
            Some(a) => a.clone(),
            None => {
                warn!("ModerationApiKey absent : sanction automod non enregistree");
                return;
            }
        }
    };
    let action = ModerationAction {
        guild_id: guild_id.to_string(),
        channel_id: action_channel_id.to_string(),
        moderator_id: moderator_id.to_string(),
        moderator_name: moderator_name.to_string(),
        target_id: target_id.to_string(),
        target_name: target_name.to_string(),
        action_type: action_type.to_string(),
        reason: reason.to_string(),
        gravity: if action_type == "warn" { Some("medium".to_string()) } else { None },
        duration: if action_type == "mute" { duration } else { None },
    };
    if let Err(e) = mod_api.log_action(&action).await {
        warn!(error = %e, target = target_id, action = action_type, "Echec enregistrement sanction automod cote moderation");
    }
}

/// Execute la sanction Discord decidee (delete/mute/ban). Helper partage par le
/// vote (finalisation) et la review 1-clic, pour une seule implementation.
/// `warn`/`ignore` = pas d'action Discord destructive.
pub(super) async fn apply_member_sanction(
    ctx: &Context,
    guild_id: Option<serenity::model::id::GuildId>,
    channel_id_str: &str,
    message_id_str: &str,
    user_id_str: &str,
    action: &str,
    mute_secs: u64,
) {
    let channel_id = match channel_id_str.parse::<u64>() {
        Ok(id) => serenity::model::id::ChannelId::new(id),
        Err(_) => return,
    };
    match action {
        "delete" | "mute" => {
            if let Ok(mid) = message_id_str.parse::<u64>() {
                let _ = channel_id
                    .delete_message(&ctx.http, serenity::model::id::MessageId::new(mid))
                    .await;
            }
            if action == "mute" {
                if let (Some(gid), Ok(uid)) = (guild_id, user_id_str.parse::<u64>()) {
                    if let Ok(mut member) = gid.member(&ctx.http, serenity::model::id::UserId::new(uid)).await {
                        let until = (std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_secs() as i64)
                            .unwrap_or(0))
                            + mute_secs as i64;
                        if let Ok(dt) = time::OffsetDateTime::from_unix_timestamp(until) {
                            let _ = member
                                .disable_communication_until_datetime(&ctx.http, serenity::model::Timestamp::from(dt))
                                .await;
                        }
                    }
                }
            }
        }
        "ban" => {
            if let (Some(gid), Ok(uid)) = (guild_id, user_id_str.parse::<u64>()) {
                if let Err(e) = gid.ban(&ctx.http, serenity::model::id::UserId::new(uid), 0).await {
                    warn!(error = %e, user = user_id_str, "Echec ban (sanction validee) -- permission BAN_MEMBERS ?");
                }
            }
        }
        "prevention" => {
            // Cran le plus leger : aucun acte destructif, juste un message public
            // de prevention (la trace est enregistree cote moderation).
            let embed = serenity::builder::CreateEmbed::new()
                .title("Prevention")
                .description(format!(
                    "<@{}> — message de prevention de la moderation. Merci d'ajuster ton comportement.",
                    user_id_str
                ))
                .color(0x3498db);
            let _ = channel_id
                .send_message(&ctx.http, serenity::builder::CreateMessage::new().embed(embed))
                .await;
        }
        _ => {}
    }
}
