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
use super::config::EmbedColors;
use super::detectors;

pub(super) const VOTE_PREFIX: &str = "amv:";
pub(super) const FINALIZE_PREFIX: &str = "amf:";

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
        'w' => "warn",
        'd' => "delete",
        'm' => "mute",
        'b' => "ban",
        _ => "ignore",
    }
}

fn action_label(s: &str) -> &'static str {
    match s {
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
    if resp.merged {
        edit_aggregated_card(ctx, &api, &resp).await;
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

    // Bouton lien : clic -> saute directement sur le message dans le salon.
    let msg_url = format!(
        "https://discord.com/channels/{}/{}/{}",
        guild_id, channel_id, message_id
    );
    let link_row = serenity::builder::CreateActionRow::Buttons(vec![
        serenity::builder::CreateButton::new_link(msg_url).label("Aller au message"),
    ]);

    let builder = serenity::builder::CreateMessage::new()
        .embed(embed)
        .components(vec![vote_buttons(&review_id), link_row]);

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
    });

    #[derive(serde::Deserialize)]
    struct CreateResp { id: String }
    let review_id = match api.post_json::<_, CreateResp>("/api/automod/reviews", &body).await {
        Ok(r) => r.id,
        Err(e) => {
            warn!(error = %e, "Echec creation review (carte manuelle)");
            return;
        }
    };

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
    let link_row = serenity::builder::CreateActionRow::Buttons(vec![
        serenity::builder::CreateButton::new_link(msg_url).label("Aller au message"),
    ]);

    let builder = serenity::builder::CreateMessage::new()
        .embed(embed)
        .components(vec![vote_buttons(&review_id), link_row]);

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

fn vote_buttons(review_id: &str) -> serenity::builder::CreateActionRow {
    use serenity::all::ButtonStyle;
    use serenity::builder::CreateButton;
    serenity::builder::CreateActionRow::Buttons(vec![
        CreateButton::new(format!("{VOTE_PREFIX}w:{review_id}")).label("Warn").style(ButtonStyle::Secondary),
        CreateButton::new(format!("{VOTE_PREFIX}d:{review_id}")).label("Delete").style(ButtonStyle::Secondary),
        CreateButton::new(format!("{VOTE_PREFIX}m:{review_id}")).label("Mute").style(ButtonStyle::Primary),
        CreateButton::new(format!("{VOTE_PREFIX}b:{review_id}")).label("Ban").style(ButtonStyle::Danger),
        CreateButton::new(format!("{VOTE_PREFIX}i:{review_id}")).label("Ignorer").style(ButtonStyle::Secondary),
    ])
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

/// Rend la liste des incidents agreges (du plus ancien au plus recent),
/// tronquee pour respecter la limite d'un field embed (1024 caracteres).
fn render_incidents(incidents: &serde_json::Value) -> String {
    let Some(arr) = incidents.as_array() else { return String::new() };
    if arr.is_empty() {
        return String::new();
    }
    let mut lines: Vec<String> = Vec::new();
    let mut total = 0usize;
    for inc in arr {
        let action = inc.get("suggested_action").and_then(|v| v.as_str()).unwrap_or("warn");
        let sc = inc.get("score").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let preview = inc.get("content_preview").and_then(|v| v.as_str()).unwrap_or("");
        let preview: String = preview.chars().take(80).collect();
        let preview = if preview.trim().is_empty() { "*(pièce jointe / embed)*".to_string() } else { preview };
        let line = format!("• **{}** ({:.1}) — {}", action_label(action), sc, preview);
        if total + line.len() + 1 > 1000 {
            lines.push("…".to_string());
            break;
        }
        total += line.len() + 1;
        lines.push(line);
    }
    lines.join("\n")
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
    let incidents = render_incidents(&resp.incidents);
    if !incidents.is_empty() {
        embed = embed.field("Detail des incidents", incidents, false);
    }
    embed = embed
        .field(VOTES_FIELD, render_votes(votes), false)
        .footer(serenity::builder::CreateEmbedFooter::new(
            "Alertes regroupees. Votez la sanction ; a l'echeance, un admin finalise.",
        ))
        .timestamp(serenity::model::Timestamp::now());
    embed
}

/// Edite la carte existante d'une review agregee : recharge le mapping Discord
/// + les votes, puis remplace l'embed (les boutons de vote sont conserves).
async fn edit_aggregated_card(ctx: &Context, api: &Arc<BaseApiClient>, resp: &ReviewResp) {
    use serenity::all::{ChannelId, MessageId};

    #[derive(serde::Deserialize)]
    struct Mapping { kind: String, channel_id: String, message_id: String }
    let mappings: Vec<Mapping> = match api.get_json(&format!("/api/discord-messages/{}", resp.id)).await {
        Ok(m) => m,
        Err(e) => { warn!(error = %e, review_id = %resp.id, "Echec fetch mapping (agregation)"); return; }
    };
    let Some(mapping) = mappings.into_iter().find(|m| m.kind == "automod_review") else { return };
    let (Ok(cid), Ok(mid)) = (mapping.channel_id.parse::<u64>(), mapping.message_id.parse::<u64>()) else { return };
    let channel_id = ChannelId::new(cid);
    let msg_id = MessageId::new(mid);

    // Recharge les votes pour les conserver dans l'embed reconstruit.
    let votes: Vec<VoteDto> = api
        .get_json(&format!("/api/automod/reviews/{}/votes", resp.id))
        .await
        .unwrap_or_default();

    let embed = aggregated_vote_embed(resp, &votes);
    // On n'edite que l'embed : les composants (boutons de vote + lien) restent.
    if let Err(e) = channel_id
        .edit_message(&ctx.http, msg_id, serenity::builder::EditMessage::new().embed(embed))
        .await
    {
        warn!(error = %e, review_id = %resp.id, "Echec edition carte agregee");
    } else {
        info!(review_id = %resp.id, incidents = resp.incident_count, "Carte agregee mise a jour");
    }
}

/// Verifie qu'un membre a le droit de voter : role configure si defini,
/// sinon permission Discord MODERATE_MEMBERS / MANAGE_MESSAGES / ADMIN.
fn can_vote(component: &serenity::model::application::ComponentInteraction, mod_role_id: Option<u64>) -> bool {
    if let (Some(role), Some(member)) = (mod_role_id, component.member.as_ref()) {
        if member.roles.iter().any(|r| r.get() == role) {
            return true;
        }
    }
    component
        .member
        .as_ref()
        .and_then(|m| m.permissions)
        .map(|p| {
            p.contains(serenity::all::Permissions::MODERATE_MEMBERS)
                || p.contains(serenity::all::Permissions::MANAGE_MESSAGES)
                || p.contains(serenity::all::Permissions::ADMINISTRATOR)
        })
        .unwrap_or(false)
}

fn can_finalize(component: &serenity::model::application::ComponentInteraction, admin_role_id: Option<u64>) -> bool {
    if let (Some(role), Some(member)) = (admin_role_id, component.member.as_ref()) {
        if member.roles.iter().any(|r| r.get() == role) {
            return true;
        }
    }
    component
        .member
        .as_ref()
        .and_then(|m| m.permissions)
        .map(|p| p.contains(serenity::all::Permissions::ADMINISTRATOR))
        .unwrap_or(false)
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

    if !can_vote(component, mod_role_id) {
        reply_ephemeral(ctx, component, "Tu n'es pas autorise a voter.").await;
        return;
    }

    let body = serde_json::json!({
        "voter_id": component.user.id.to_string(),
        "voter_name": component.user.name,
        "vote_action": vote_action,
    });
    let votes: Vec<VoteDto> = match api
        .post_json(&format!("/api/automod/reviews/{review_id}/vote"), &body)
        .await
    {
        Ok(v) => v,
        Err(e) => {
            warn!(error = %e, review_id, "Echec enregistrement vote");
            reply_ephemeral(ctx, component, "Le vote n'a pas pu etre enregistre (vote clos ?).").await;
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
    let row = serenity::builder::CreateActionRow::Buttons(vec![finalize_btn]);

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

    if !can_finalize(component, admin_role_id) {
        reply_ephemeral(ctx, component, "Seul un administrateur peut finaliser.").await;
        return;
    }

    // Recupere la review (verdict + cible) depuis l'API.
    #[derive(serde::Deserialize)]
    struct ReviewDto {
        guild_id: String,
        channel_id: String,
        message_id: String,
        user_id: String,
        user_name: String,
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

    // Persiste la resolution cote API (source discord).
    let resolve_body = serde_json::json!({
        "applied_action": decided,
        "resolved_by_id": component.user.id.to_string(),
        "resolved_by_name": component.user.name,
        "source": "discord",
    });
    if let Err(e) = api
        .post_json::<_, serde_json::Value>(&format!("/api/automod/reviews/{review_id}/resolve"), &resolve_body)
        .await
    {
        warn!(error = %e, review_id, "Echec resolve finalize");
        reply_ephemeral(ctx, component, "Echec de l'enregistrement (deja finalise ?).").await;
        return;
    }

    // Verdict warn : on cree une vraie infraction via le module moderation
    // (gRPC log_action, comme /warn) pour qu'elle compte dans l'historique /
    // l'escalade. Les autres sanctions Discord sont gerees par execute_sanction.
    if decided == "warn" {
        log_warn_to_moderation(ctx, component, &review.guild_id, &review.user_id, &review.user_name).await;
    }

    // Execute la sanction Discord (delete/mute/ban ; warn deja loggue ci-dessus).
    let mute_secs = BaseApiClient::config_u64(&config, "mute_duration_secs", super::DEFAULT_MUTE_DURATION_SECS);
    execute_sanction(ctx, component, &review.channel_id, &review.message_id, &review.user_id, &decided, mute_secs).await;

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
                serenity::builder::CreateInteractionResponseMessage::new().embed(finalized).components(vec![]),
            ),
        )
        .await;
    info!(review_id, action = %decided, admin = %component.user.name, "Vote automod finalise");
}

/// Enregistre une infraction warn via le module moderation (gRPC log_action),
/// de sorte que le warn issu d'un vote compte dans l'historique et l'escalade
/// au meme titre qu'un /warn manuel. L'admin qui finalise est le "moderateur".
async fn log_warn_to_moderation(
    ctx: &Context,
    component: &serenity::model::application::ComponentInteraction,
    guild_id: &str,
    target_id: &str,
    target_name: &str,
) {
    use crate::modules::moderation::api_client::ModerationAction;
    use crate::modules::moderation::ModerationApiKey;

    let mod_api = {
        let data = ctx.data.read().await;
        match data.get::<ModerationApiKey>() {
            Some(a) => a.clone(),
            None => {
                warn!("ModerationApiKey absent : warn vote non enregistre");
                return;
            }
        }
    };
    let action = ModerationAction {
        guild_id: guild_id.to_string(),
        channel_id: component.channel_id.to_string(),
        moderator_id: component.user.id.to_string(),
        moderator_name: component.user.name.clone(),
        target_id: target_id.to_string(),
        target_name: target_name.to_string(),
        action_type: "warn".to_string(),
        reason: "Sanction validee par vote des moderateurs (AutoMod)".to_string(),
        gravity: Some("medium".to_string()),
        duration: None,
    };
    if let Err(e) = mod_api.log_action(&action).await {
        warn!(error = %e, target = target_id, "Echec enregistrement warn (vote) cote moderation");
    }
}

/// Execute la sanction Discord decidee (warn/delete/mute/ban/ignore).
async fn execute_sanction(
    ctx: &Context,
    component: &serenity::model::application::ComponentInteraction,
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
                if let (Some(gid), Ok(uid)) = (component.guild_id, user_id_str.parse::<u64>()) {
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
            if let (Some(gid), Ok(uid)) = (component.guild_id, user_id_str.parse::<u64>()) {
                if let Err(e) = gid.ban(&ctx.http, serenity::model::id::UserId::new(uid), 0).await {
                    warn!(error = %e, user = user_id_str, "Echec ban via vote -- permission BAN_MEMBERS ?");
                }
            }
        }
        // "warn" : trace uniquement (pas d'action Discord destructive).
        // "ignore" : rien.
        _ => {}
    }
}
