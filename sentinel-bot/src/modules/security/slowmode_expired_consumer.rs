//! Phase 5H — Consumer Redis pour `slowmode_expired` publie par le
//! worker `expire_slowmode`. Restaure le rate_limit_per_user original
//! sur chaque salon.

use serenity::all::{ChannelId, Context, GuildId};
use serenity::builder::EditChannel;
use std::str::FromStr;
use tracing::{info, warn};

use super::SlowmodeKey;

pub fn spawn(ctx: Context) {
    tokio::spawn(async move {
        let consumer = crate::shared::event_bus::default_consumer_name();
        crate::shared::event_bus::listen_stream_group(
            "security-bot-slowmode-expired".to_string(),
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
    if event.get("event").and_then(|v| v.as_str()) != Some("slowmode_expired") {
        return;
    }
    let data = match event.get("data") {
        Some(d) => d,
        None => return,
    };
    let guild_id_str = data.get("guild_id").and_then(|v| v.as_str()).unwrap_or("");
    let guild_id = match u64::from_str(guild_id_str) {
        Ok(g) => GuildId::new(g),
        Err(_) => return,
    };
    let states = match data.get("previous_states").and_then(|v| v.as_array()) {
        Some(a) => a,
        None => return,
    };

    let mut restored = 0usize;
    for entry in states {
        let ch_str = entry
            .get("channel_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let rate = entry.get("rate").and_then(|v| v.as_u64()).unwrap_or(0) as u16;
        let channel_id = match u64::from_str(ch_str) {
            Ok(c) => ChannelId::new(c),
            Err(_) => continue,
        };
        let edit = EditChannel::new().rate_limit_per_user(rate);
        if let Err(e) = channel_id.edit(&ctx.http, edit).await {
            warn!(error = %e, channel = %channel_id, "Restore slowmode echoue");
            continue;
        }
        restored += 1;
    }

    let bot_data = ctx.data.read().await;
    if let Some(sm) = bot_data.get::<SlowmodeKey>() {
        sm.deactivate_with_http(&ctx.http, guild_id).await;
    }

    info!(guild = %guild_id, restored, "Slowmode anti-raid expire restaure (event worker)");
}
