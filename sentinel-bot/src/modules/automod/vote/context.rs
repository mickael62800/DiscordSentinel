//! Recuperation et rendu du contexte conversationnel (messages avant/apres) et
//! de la liste des incidents agreges.

use serenity::model::channel::Message;
use serenity::prelude::*;
use tracing::warn;

use super::super::review;
use super::labels::action_label;

/// Recupere jusqu'a `n` messages precedant le message signale et les rend
/// en bloc chronologique (du plus ancien au plus recent). Tronque pour
/// respecter la limite d'un field embed (1024 caracteres).
pub(super) async fn fetch_context_before(ctx: &Context, msg: &Message, n: u8) -> String {
    if n == 0 {
        return String::new();
    }
    let limit = n.min(25);
    let before = match msg
        .channel_id
        .messages(
            &ctx.http,
            serenity::builder::GetMessages::new()
                .before(msg.id)
                .limit(limit),
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
        let content = review::sanitize_embed_content(&m.content, 120);
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

/// Variante de `fetch_context_before` par identifiants (salon + message), sans
/// objet `Message` — utilisee pour re-construire le contexte d'une carte
/// agregee autour du dernier incident.
pub(super) async fn fetch_context_before_ids(
    ctx: &Context,
    channel_id: serenity::model::id::ChannelId,
    message_id: serenity::model::id::MessageId,
    n: u8,
) -> String {
    if n == 0 {
        return String::new();
    }
    let limit = n.min(25);
    let before = match channel_id
        .messages(
            &ctx.http,
            serenity::builder::GetMessages::new()
                .before(message_id)
                .limit(limit),
        )
        .await
    {
        Ok(m) => m,
        Err(e) => {
            warn!(error = %e, "Echec recuperation contexte agrege (messages avant)");
            return String::new();
        }
    };
    let mut lines: Vec<String> = Vec::new();
    let mut total = 0usize;
    for m in before.iter().rev() {
        let content = review::sanitize_embed_content(&m.content, 120);
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

/// Variante de `fetch_context_after` par identifiants (salon + message), pour
/// recuperer les N messages POSTERIEURS a l'infraction (salon de discussion).
pub(super) async fn fetch_context_after_ids(
    ctx: &Context,
    channel_id: serenity::model::id::ChannelId,
    message_id: serenity::model::id::MessageId,
    n: u8,
) -> String {
    if n == 0 {
        return String::new();
    }
    let limit = n.min(25);
    let after = match channel_id
        .messages(
            &ctx.http,
            serenity::builder::GetMessages::new()
                .after(message_id)
                .limit(limit),
        )
        .await
    {
        Ok(m) => m,
        Err(e) => {
            warn!(error = %e, "Echec recuperation contexte apres (discussion)");
            return String::new();
        }
    };
    // L'API renvoie du plus recent au plus ancien -> inverse pour l'ordre chrono.
    let mut lines: Vec<String> = Vec::new();
    let mut total = 0usize;
    for m in after.iter().rev() {
        let content = review::sanitize_embed_content(&m.content, 120);
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

/// Rend la liste numerotee des infractions agregees (incidents) pour le message
/// d'ancrage du salon de discussion : `1. [Action] contenu — raison`. Tronque
/// pour rester sous la limite d'un field embed (1024).
pub(super) fn render_incident_list(incidents: &serde_json::Value, _count: i32) -> String {
    let arr = match incidents.as_array() {
        Some(a) if !a.is_empty() => a,
        _ => return String::new(),
    };
    let mut lines: Vec<String> = Vec::new();
    let mut total = 0usize;
    for (i, inc) in arr.iter().enumerate() {
        let content = inc
            .get("content_preview")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let reason = inc.get("reason").and_then(|v| v.as_str()).unwrap_or("");
        let action = inc
            .get("suggested_action")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let c = review::sanitize_embed_content(content, 100);
        let c = if c.trim().is_empty() {
            "*(pièce jointe / embed)*".to_string()
        } else {
            c
        };
        let line = if reason.is_empty() {
            format!("**{}.** [{}] {}", i + 1, action_label(action), c)
        } else {
            format!(
                "**{}.** [{}] {} — _{}_",
                i + 1,
                action_label(action),
                c,
                reason
            )
        };
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
pub(super) async fn fetch_context_after(ctx: &Context, msg: &Message, n: u8) -> String {
    if n == 0 {
        return String::new();
    }
    let limit = n.min(25);
    let after = match msg
        .channel_id
        .messages(
            &ctx.http,
            serenity::builder::GetMessages::new()
                .after(msg.id)
                .limit(limit),
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
        let content = review::sanitize_embed_content(&m.content, 120);
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
