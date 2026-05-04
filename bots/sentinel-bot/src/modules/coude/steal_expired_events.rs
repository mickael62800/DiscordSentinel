//! Phase 5 — Consumer Redis stream pour `coude:steal_expired`.
//!
//! Quand le worker `expire_steals` (sentinel-worker, domaine coude)
//! detecte qu'une tentative /voler a depasse sa fenetre de defense
//! (60s), il publie cet event sur `sentinel:events`. Ce consumer le
//! recoit et execute la **resolution AFK** comme l'ancien
//! `tokio::spawn(sleep 60s)` de voler.rs — mais cette fois c'est
//! resilient aux redemarrages du bot.
//!
//! Pattern aligne sur `daily_chaos_events.rs` et `tournament_events.rs`.

use std::str::FromStr;

use serenity::all::{ChannelId, Context, EditMessage, MessageId};
use tracing::{info, warn};
use uuid::Uuid;

use crate::modules::coude::commands::voler::resolve_steal_attempt;
use crate::modules::coude::catalog::CatalogCacheKey;
use crate::modules::coude::load_guild_config;
use crate::modules::coude::taunts_dispatch;
use crate::modules::coude::GameApiKey;

/// Spawn le consumer durable. Appele une seule fois au `ready`.
pub fn spawn(ctx: Context) {
    tokio::spawn(async move {
        let consumer = sentinel_shared::event_bus::default_consumer_name();
        sentinel_shared::event_bus::listen_stream_group(
            "coude-bot-steal-expired".to_string(),
            consumer,
            move |payload_json| {
                let ctx = ctx.clone();
                async move {
                    handle_event(&ctx, &payload_json).await;
                }
            },
        )
        .await;
    });
}

async fn handle_event(ctx: &Context, payload_json: &str) {
    let event: serde_json::Value = match serde_json::from_str(payload_json) {
        Ok(v) => v,
        Err(_) => return,
    };
    if event.get("event").and_then(|v| v.as_str()) != Some("coude:steal_expired") {
        return;
    }

    let data = match event.get("data") {
        Some(d) => d,
        None => return,
    };

    let attempt_id_str = data.get("attempt_id").and_then(|v| v.as_str()).unwrap_or("");
    let guild_id = data.get("guild_id").and_then(|v| v.as_str()).unwrap_or("");
    let thief_id = data.get("thief_id").and_then(|v| v.as_str()).unwrap_or("");
    let target_id = data.get("target_id").and_then(|v| v.as_str()).unwrap_or("");
    let message_id_str = data.get("message_id").and_then(|v| v.as_str()).unwrap_or("");
    let channel_id_str = data.get("channel_id").and_then(|v| v.as_str()).unwrap_or("");

    if attempt_id_str.is_empty()
        || guild_id.is_empty()
        || thief_id.is_empty()
        || target_id.is_empty()
        || message_id_str.is_empty()
        || channel_id_str.is_empty()
    {
        warn!("coude:steal_expired payload incomplet, skip");
        return;
    }

    let attempt_id = match Uuid::parse_str(attempt_id_str) {
        Ok(u) => u,
        Err(_) => {
            warn!(attempt_id = %attempt_id_str, "attempt_id invalide");
            return;
        }
    };

    let channel_id = match u64::from_str(channel_id_str) {
        Ok(c) => ChannelId::new(c),
        Err(_) => return,
    };
    let message_id = match u64::from_str(message_id_str) {
        Ok(m) => MessageId::new(m),
        Err(_) => return,
    };

    info!(
        attempt_id = %attempt_id,
        thief = %thief_id,
        target = %target_id,
        "coude:steal_expired -> resolution AFK"
    );

    // Si la victime a clique le bouton entre-temps mais que le worker
    // a quand meme passe en expired (race), on respecte tout de meme la
    // resolution AFK — le row a deja ete UPDATE en 'expired' cote SQL,
    // donc le bouton ne fera plus rien (bot le check via mark_steal_defended).
    let config = load_guild_config(ctx, guild_id).await;
    let failure_penalty_pct = config.steal_failure_penalty_pct();

    let bot_data = ctx.data.read().await;
    let api = match bot_data.get::<GameApiKey>() {
        Some(a) => a,
        None => {
            warn!("GameApiKey absent du TypeMap, skip");
            return;
        }
    };
    let catalog = match bot_data.get::<CatalogCacheKey>() {
        Some(c) => c.clone(),
        None => {
            warn!("CatalogCacheKey absent du TypeMap, skip");
            return;
        }
    };

    let thief_player = match api.get_or_create_player(guild_id, thief_id, "").await {
        Ok(p) => p,
        Err(e) => {
            warn!(error = %e, "Echec get_or_create_player thief (steal_expired)");
            return;
        }
    };
    let target_player = match api.get_or_create_player(guild_id, target_id, "").await {
        Ok(p) => p,
        Err(e) => {
            warn!(error = %e, "Echec get_or_create_player target (steal_expired)");
            return;
        }
    };

    let (result_embed, taunt_events) = resolve_steal_attempt(
        api,
        &catalog,
        guild_id,
        thief_id,
        target_id,
        &thief_player,
        &target_player,
        true, // AFK
        failure_penalty_pct,
    )
    .await;

    // Edit le message d'alerte original (retire le bouton).
    if let Err(e) = channel_id
        .edit_message(
            &ctx.http,
            message_id,
            EditMessage::new().embed(result_embed).components(vec![]),
        )
        .await
    {
        warn!(error = %e, "Echec edit_message (steal_expired)");
    }

    // Phase 9 Part D : dispatch les taunt events (IO pur).
    if !taunt_events.is_empty() {
        if let Ok(guild_id_u64) = guild_id.parse::<u64>() {
            let gid = serenity::all::GuildId::new(guild_id_u64);
            taunts_dispatch::dispatch_all(ctx, gid, &taunt_events).await;
        }
    }

    // Marque resolved (idempotent, fire-and-forget).
    api.mark_steal_resolved(attempt_id).await;
}
