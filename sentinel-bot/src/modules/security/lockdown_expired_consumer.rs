//! Phase 5G — Consumer Redis pour `lockdown_expired` publie par le
//! worker `expire_lockdown`.
//!
//! Recoit les saved_states JSON, desserialise et restaure les
//! permission_overwrites Discord.

use serenity::all::{ChannelId, Context, GuildId, PermissionOverwriteType};
use std::str::FromStr;
use tracing::{info, warn};

use super::detectors::lockdown::deserialize_saved_state;
use super::LockdownKey;

pub fn spawn(ctx: Context) {
    tokio::spawn(async move {
        let consumer = crate::shared::event_bus::default_consumer_name();
        crate::shared::event_bus::listen_stream_group(
            "security-bot-lockdown-expired".to_string(),
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
    if event.get("event").and_then(|v| v.as_str()) != Some("lockdown_expired") {
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
    let saved = match data.get("saved_states").and_then(|v| v.as_array()) {
        Some(a) => a,
        None => return,
    };

    let everyone_role = serenity::model::id::RoleId::new(guild_id.get());
    let mut restored = 0usize;
    for entry in saved {
        let (channel_id_raw, original) = match deserialize_saved_state(entry) {
            Some(x) => x,
            None => continue,
        };
        let channel_id = ChannelId::new(channel_id_raw);
        match original {
            Some(ow) => {
                if let Err(e) = channel_id.create_permission(&ctx.http, ow).await {
                    warn!(error = %e, channel = %channel_id, "Restore permission lockdown");
                    continue;
                }
            }
            None => {
                if let Err(e) = channel_id
                    .delete_permission(&ctx.http, PermissionOverwriteType::Role(everyone_role))
                    .await
                {
                    warn!(error = %e, channel = %channel_id, "Delete overwrite lockdown");
                    continue;
                }
            }
        }
        restored += 1;
    }

    // Cleanup tracker RAM si encore present (defensif).
    let bot_data = ctx.data.read().await;
    if let Some(lk) = bot_data.get::<LockdownKey>() {
        // Ne expose pas un remove direct, mais deactivate_with_http le
        // fait deja (et c'est idempotent meme si on l'appelle apres).
        lk.deactivate_with_http(&ctx.http, guild_id).await;
    }

    info!(
        guild = %guild_id,
        restored,
        "Lockdown expire restaure (event worker)"
    );
}
