//! Handlers des events Redis : verdict (`automod_review_decided`) et expiration
//! de carte (`automod_card_expired`).

use serenity::prelude::*;
use tracing::{info, warn};

use super::labels::action_label;
use super::{CLOSE_PREFIX, FINALIZE_PREFIX};

/// Event Redis `automod_review_decided` : edite la carte (verdict) et
/// ajoute le bouton admin de finalisation.
pub(crate) async fn handle_decided_event(ctx: &Context, payload: &str) {
    use serenity::all::{ChannelId, GetMessages, MessageId};
    let event: serde_json::Value = match serde_json::from_str(payload) {
        Ok(v) => v,
        Err(_) => return,
    };
    if event.get("event").and_then(|e| e.as_str()) != Some("automod_review_decided") {
        return;
    }
    let data = match event.get("data") {
        Some(d) => d,
        None => return,
    };
    let action_id = data.get("action_id").and_then(|v| v.as_str()).unwrap_or("");
    if action_id.is_empty() {
        return;
    }
    let decided_action = data
        .get("decided_action")
        .and_then(|v| v.as_str())
        .unwrap_or("ignore");
    let quorum_met = data
        .get("quorum_met")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let total_votes = data
        .get("total_votes")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    let grpc = {
        let d = ctx.data.read().await;
        match d.get::<crate::shared::grpc_client::GrpcClientKey>() {
            Some(g) => g.clone(),
            None => return,
        }
    };
    let mappings = match crate::sync::list_action_messages(&grpc, action_id).await {
        Ok(l) => l,
        Err(e) => {
            warn!(error = %e, action_id, "Echec fetch mapping (decided)");
            return;
        }
    };
    let mapping = match mappings.into_iter().find(|m| m.kind == "automod_review") {
        Some(m) => m,
        None => return,
    };
    let channel_id = match mapping.channel_id.parse::<u64>() {
        Ok(v) => ChannelId::new(v),
        Err(_) => return,
    };
    let msg_id = match mapping.message_id.parse::<u64>() {
        Ok(v) => MessageId::new(v),
        Err(_) => return,
    };

    let verdict = if !quorum_met {
        format!("Quorum non atteint ({total_votes} votes) -> aucune sanction. Un admin doit clore.")
    } else {
        format!(
            "Verdict : **{}** ({total_votes} votes). En attente de finalisation par un admin.",
            action_label(decided_action)
        )
    };

    let finalize_btn =
        serenity::builder::CreateButton::new(format!("{FINALIZE_PREFIX}{action_id}"))
            .label(format!("Finaliser ({})", action_label(decided_action)))
            .style(serenity::all::ButtonStyle::Success);
    let close_btn = serenity::builder::CreateButton::new(format!("{CLOSE_PREFIX}{action_id}"))
        .label("🚫 Clore (ignorer)")
        .style(serenity::all::ButtonStyle::Danger);
    let row = serenity::builder::CreateActionRow::Buttons(vec![finalize_btn, close_btn]);

    if let Ok(messages) = channel_id
        .messages(&ctx.http, GetMessages::new().limit(1).around(msg_id))
        .await
    {
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
                        serenity::builder::EditMessage::new()
                            .embed(new_embed)
                            .components(vec![row]),
                    )
                    .await;
            }
        }
    }
    info!(
        action_id,
        decided_action, quorum_met, "Carte vote editee (decided)"
    );
}

/// Event Redis `automod_card_expired` (worker 24h) : supprime le message
/// Discord d'une carte close vieille de plus d'un mois. La review + le
/// transcript restent en DB (trace web).
pub(crate) async fn handle_card_expired_event(ctx: &Context, payload: &str) {
    use serenity::all::{ChannelId, MessageId};
    let event: serde_json::Value = match serde_json::from_str(payload) {
        Ok(v) => v,
        Err(_) => return,
    };
    if event.get("event").and_then(|e| e.as_str()) != Some("automod_card_expired") {
        return;
    }
    let data = match event.get("data") {
        Some(d) => d,
        None => return,
    };
    let cid = data
        .get("channel_id")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let mid = data
        .get("message_id")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let (Ok(c), Ok(m)) = (cid.parse::<u64>(), mid.parse::<u64>()) else {
        return;
    };
    match ChannelId::new(c)
        .delete_message(&ctx.http, MessageId::new(m))
        .await
    {
        Ok(_) => info!(
            channel = c,
            message = m,
            "Carte automod close supprimee (>1 mois)"
        ),
        Err(e) => {
            // 404 = deja supprimee : on ignore.
            let s = e.to_string();
            if !s.contains("404") && !s.contains("Unknown Message") {
                warn!(error = %e, "Echec suppression carte automod expiree");
            }
        }
    }
}
