//! Consumer stream : ferme les channels Discord des tables marquees AFK
//! par le `blackjack-cleanup-worker` (Phase 6A).
//!
//! Flow :
//! 1. Le worker query `blackjack_tables WHERE status='open' AND
//!    last_activity < NOW() - 30min`, UPDATE en 'closed' (source de verite DB),
//!    publie un event `blackjack_table_afk` sur `sentinel:events`.
//! 2. Le bot consume cet event via `event_bus::listen_stream_group`,
//!    envoie un message de notification dans le channel puis le supprime,
//!    et retire l'entree de son `ChannelManager` local.

use std::sync::Arc;

use serenity::builder::{CreateEmbed, CreateMessage};
use serenity::model::id::ChannelId;
use serenity::prelude::*;
use tracing::{info, warn};

use super::ChannelManagerKey;

/// Spawn le consumer durable. Appele une seule fois au `ready`.
pub fn spawn(ctx: Context) {
    tokio::spawn(async move {
        let consumer = crate::shared::event_bus::default_consumer_name();
        crate::shared::event_bus::listen_stream_group(
            "blackjack-bot-cleanup".to_string(),
            consumer,
            move |payload_json| {
                let ctx = ctx.clone();
                async move {
                    handle_cleanup_event(&ctx, &payload_json).await;
                }
            },
        )
        .await;
    });
}

async fn handle_cleanup_event(ctx: &Context, payload_json: &str) {
    // Filtrage event type — on ignore les autres events qui passent sur
    // la meme stream (moderation_action, watched_users_refreshed, etc.)
    let event: serde_json::Value = match serde_json::from_str(payload_json) {
        Ok(v) => v,
        Err(_) => return,
    };
    let event_type = event.get("event").and_then(|v| v.as_str()).unwrap_or("");
    if event_type != "blackjack_table_afk" {
        return;
    }

    let data = event.get("data").cloned().unwrap_or_default();
    let channel_id_str = data.get("channel_id").and_then(|v| v.as_str()).unwrap_or("");
    let idle_minutes = data.get("idle_minutes").and_then(|v| v.as_i64()).unwrap_or(0);

    let channel_id = match channel_id_str.parse::<u64>() {
        Ok(id) => ChannelId::new(id),
        Err(_) => {
            warn!(channel_id = channel_id_str, "blackjack_table_afk: channel_id invalide");
            return;
        }
    };

    // Notification dans le channel avant suppression (best-effort)
    let embed = CreateEmbed::new()
        .title("\u{23f0} Table fermee — Inactivite")
        .description(format!(
            "Cette table de blackjack a ete fermee apres **{} minutes** d'inactivite.",
            idle_minutes
        ))
        .color(0x95A5A6);
    let _ = channel_id
        .send_message(&ctx.http, CreateMessage::new().embed(embed))
        .await;

    // Petit delai pour que l'utilisateur voie le message avant la suppression
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    // Suppression du channel Discord (idempotente : 404 si deja supprime)
    if let Err(e) = channel_id.delete(&ctx.http).await {
        warn!(error = %e, channel_id = %channel_id, "Echec suppression channel blackjack AFK");
    } else {
        info!(channel_id = %channel_id, idle_minutes, "Table blackjack AFK supprimee (event)");
    }

    // Retirer l'entree du ChannelManager local
    let ctx_data = ctx.data.read().await;
    let mgr = match ctx_data.get::<ChannelManagerKey>() {
        Some(m) => Arc::clone(m),
        None => return,
    };
    drop(ctx_data);

    if let Some((user_id, _table)) = mgr.find_by_channel(channel_id) {
        mgr.remove(user_id);
    }
}
