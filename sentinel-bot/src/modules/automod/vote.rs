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

use std::collections::HashMap;

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
    });

    #[derive(serde::Deserialize)]
    struct CreateResp { id: String }
    let review_id = match api.post_json::<_, CreateResp>("/api/automod/reviews", &body).await {
        Ok(r) => r.id,
        Err(e) => {
            warn!(error = %e, "Echec creation review vote (sync degrade)");
            return;
        }
    };

    // 2. Construire la carte.
    let embed = vote_embed(
        &user_id, &msg.author.name, &channel_id, score, &content_preview, reason, flags,
        suggested_str, &deadline, &HashMap::new(),
    );
    let row = vote_buttons(&review_id);

    let builder = serenity::builder::CreateMessage::new()
        .embed(embed)
        .components(vec![row]);

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
    info!(review_id, "Carte de vote automod postee");
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
    tally: &HashMap<String, usize>,
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
        .field("Votes", render_tally(tally), false)
        .footer(serenity::builder::CreateEmbedFooter::new(
            "Votez la sanction. A l'echeance, un admin finalise.",
        ))
        .timestamp(serenity::model::Timestamp::now())
}

fn render_tally(tally: &HashMap<String, usize>) -> String {
    let order = ["warn", "delete", "mute", "ban", "ignore"];
    let parts: Vec<String> = order
        .iter()
        .map(|a| format!("{} : **{}**", action_label(a), tally.get(*a).copied().unwrap_or(0)))
        .collect();
    parts.join("\n")
}

#[derive(serde::Deserialize)]
struct VoteDto {
    #[allow(dead_code)]
    voter_id: String,
    #[allow(dead_code)]
    voter_name: String,
    vote_action: String,
}

fn tally_from(votes: &[VoteDto]) -> HashMap<String, usize> {
    let mut m = HashMap::new();
    for v in votes {
        *m.entry(v.vote_action.clone()).or_insert(0) += 1;
    }
    m
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

    // Met a jour le champ Votes de la carte (l'embed existant) sans tout reconstruire.
    let tally = tally_from(&votes);
    if let Some(existing) = component.message.embeds.first() {
        let updated = serenity::builder::CreateEmbed::from(existing.clone());
        // On reconstruit uniquement le champ Votes via un nouvel embed base sur l'ancien.
        // Serenity ne permet pas d'editer un field isole -> on re-set tous les fields
        // est complexe ; on prefere ajouter le tally en description courte.
        let updated = updated.field("Votes (maj)", render_tally(&tally), false);
        let _ = component
            .create_response(
                &ctx.http,
                serenity::builder::CreateInteractionResponse::UpdateMessage(
                    serenity::builder::CreateInteractionResponseMessage::new().embed(updated),
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

    // Execute la sanction Discord.
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
