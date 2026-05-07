//! Phase 5F — Consumer Redis pour `quarantine_expired` publie par le
//! worker `kick_expired_quarantine`. Kick le user et nettoie le
//! tracker RAM.
//!
//! Avant : background.rs avait une boucle 30s qui scannait
//! `QuarantineManager.expired_users()` et kickait. Si le bot
//! redemarrait, le tracker RAM etait perdu et personne ne kickait.
//!
//! Maintenant : la persistance est en DB (`security_quarantine_pending`),
//! le worker scanne et publie l'event quand le timer expire. Resilient.

use serenity::all::{Context, GuildId, UserId};
use std::str::FromStr;
use tracing::{info, warn};

use super::{CaptchaPendingKey, QuarantineKey};

pub fn spawn(ctx: Context) {
    tokio::spawn(async move {
        let consumer = crate::shared::event_bus::default_consumer_name();
        crate::shared::event_bus::listen_stream_group(
            "security-bot-quarantine-expired".to_string(),
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
    if event.get("event").and_then(|v| v.as_str()) != Some("quarantine_expired") {
        return;
    }
    let data = match event.get("data") {
        Some(d) => d,
        None => return,
    };
    let guild_id_str = data.get("guild_id").and_then(|v| v.as_str()).unwrap_or("");
    let user_id_str = data.get("user_id").and_then(|v| v.as_str()).unwrap_or("");
    if guild_id_str.is_empty() || user_id_str.is_empty() {
        return;
    }
    let guild_id = match u64::from_str(guild_id_str) {
        Ok(g) => GuildId::new(g),
        Err(_) => return,
    };
    let user_id = match u64::from_str(user_id_str) {
        Ok(u) => UserId::new(u),
        Err(_) => return,
    };

    if let Err(e) = guild_id.kick(&ctx.http, user_id).await {
        warn!(error = %e, guild = %guild_id, user = %user_id, "Impossible de kick (quarantine_expired)");
    } else {
        info!(guild = %guild_id, user = %user_id, "Utilisateur kick (captcha timeout via worker)");
    }

    // Cleanup RAM trackers (defensif).
    let bot_data = ctx.data.read().await;
    if let Some(q) = bot_data.get::<QuarantineKey>() {
        q.remove_tracking(guild_id, user_id);
    }
    if let Some(cp) = bot_data.get::<CaptchaPendingKey>() {
        cp.remove(guild_id, user_id);
    }
}
