//! Consumer stream : dispatche les TauntEvents emis par le job
//! `daily_chaos` du coude-worker (Migration #5).
//!
//! Flow :
//! 1. `coude-worker::jobs::daily_chaos` declenche un transfert victime→
//!    gagnant via `ManageWalletUseCase::transfer`, qui detecte faillite
//!    (cote victime) et jackpot (cote gagnant). Il XADD l'event
//!    `daily_chaos_taunts` sur `sentinel:events` avec la liste de
//!    TauntEvents.
//! 2. Ce consumer (group `coude-bot-daily-chaos`) lit l'event, parse la
//!    liste de taunts et delegue a `taunts_dispatch::dispatch_all` pour
//!    poster le message dans le channel dedie + renommer le membre.
//!
//! Meme pattern que `tournament_events.rs` (embed tournoi hebdo).

use std::str::FromStr;

use serenity::all::{Context, GuildId};
use tracing::{info, warn};

use crate::modules::coude::api_client::TauntEvent;
use crate::modules::coude::taunts_dispatch;

/// Spawn le consumer durable. Appele une seule fois au `ready`.
pub fn spawn(ctx: Context) {
    tokio::spawn(async move {
        let consumer = sentinel_shared::event_bus::default_consumer_name();
        sentinel_shared::event_bus::listen_stream_group(
            "coude-bot-daily-chaos".to_string(),
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
    let event_type = event.get("event").and_then(|v| v.as_str()).unwrap_or("");
    if event_type != "daily_chaos_taunts" {
        return;
    }

    let data = match event.get("data") {
        Some(d) => d,
        None => return,
    };

    let guild_id_str = data.get("guild_id").and_then(|v| v.as_str()).unwrap_or("");
    if guild_id_str.is_empty() {
        warn!("daily_chaos_taunts sans guild_id, skip");
        return;
    }

    let guild_id = match u64::from_str(guild_id_str) {
        Ok(n) => GuildId::new(n),
        Err(_) => {
            warn!(guild_id = %guild_id_str, "daily_chaos_taunts : guild_id invalide");
            return;
        }
    };

    let empty: Vec<serde_json::Value> = Vec::new();
    let taunts_json = data
        .get("taunts")
        .and_then(|v| v.as_array())
        .unwrap_or(&empty);

    let taunts: Vec<TauntEvent> = taunts_json
        .iter()
        .filter_map(parse_taunt)
        .collect();

    if taunts.is_empty() {
        return;
    }

    info!(
        guild_id = %guild_id,
        count = taunts.len(),
        "daily_chaos_taunts recu, dispatch"
    );

    taunts_dispatch::dispatch_all(ctx, guild_id, &taunts).await;
}

fn parse_taunt(v: &serde_json::Value) -> Option<TauntEvent> {
    let channel_id = v.get("channel_id").and_then(|x| x.as_str())?.to_string();
    let target_user_id = v
        .get("target_user_id")
        .and_then(|x| x.as_str())?
        .to_string();
    let message = v.get("message").and_then(|x| x.as_str())?.to_string();
    let nickname_suffix = v
        .get("nickname_suffix")
        .and_then(|x| x.as_str())?
        .to_string();
    let streak_kind = v.get("streak_kind").and_then(|x| x.as_str())?.to_string();
    let streak_value = v
        .get("streak_value")
        .and_then(|x| x.as_i64())
        .unwrap_or(0) as i32;

    Some(TauntEvent {
        channel_id,
        target_user_id,
        message,
        nickname_suffix,
        streak_kind,
        streak_value,
    })
}
