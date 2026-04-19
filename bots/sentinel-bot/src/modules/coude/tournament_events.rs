//! Consumer stream : poste l'embed "Tournoi hebdo cloture" dans Discord apres
//! qu'une instance de coude-worker ait resolu le tournoi et publie l'event
//! `tournament_resolved` sur `sentinel:events`.
//!
//! Flow :
//! 1. `coude-worker::jobs::resolve_tournament` inserte la ligne resolved,
//!    credite le gagnant, puis XADD `tournament_resolved` avec winner + top5.
//! 2. Ce consumer (group `coude-bot-tournament`) lit l'event, recupere le
//!    `tournament_channel_id` (fallback `channel_activites`) depuis la
//!    guild_config et post l'embed.

use std::str::FromStr;

use serenity::all::{ChannelId, Context};
use serenity::builder::{CreateEmbed, CreateEmbedFooter, CreateMessage};
use tracing::{info, warn};

use sentinel_shared::heartbeat::ApiClientKey;

use crate::modules::coude::guild_config::CoudeConfig;

/// Spawn le consumer durable. Appele une seule fois au `ready`.
pub fn spawn(ctx: Context) {
    tokio::spawn(async move {
        let consumer = sentinel_shared::event_bus::default_consumer_name();
        sentinel_shared::event_bus::listen_stream_group(
            "coude-bot-tournament".to_string(),
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
    if event_type != "tournament_resolved" {
        return;
    }

    let data = match event.get("data") {
        Some(d) => d,
        None => return,
    };

    let guild_id = data.get("guild_id").and_then(|v| v.as_str()).unwrap_or("");
    let winner_id = data
        .get("winner_user_id")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let winner_username = data
        .get("winner_username")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let prize = data
        .get("prize_amount")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    let prize_pct = data.get("prize_pct").and_then(|v| v.as_i64()).unwrap_or(0);
    let week_start = data
        .get("week_start")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let empty_vec = Vec::new();
    let top5 = data
        .get("top5")
        .and_then(|v| v.as_array())
        .unwrap_or(&empty_vec);

    if guild_id.is_empty() {
        warn!("tournament_resolved sans guild_id, skip");
        return;
    }

    // Charger la config Coude (channel de post).
    let config = {
        let data_read = ctx.data.read().await;
        let api = match data_read.get::<ApiClientKey>() {
            Some(a) => a.clone(),
            None => {
                warn!("ApiClientKey absent, tournament_resolved skip");
                return;
            }
        };
        drop(data_read);
        CoudeConfig::load(&api, guild_id).await
    };

    let channel_id_str = match config.channel_tournament() {
        Some(id) if !id.is_empty() => id,
        _ => {
            info!(
                guild_id,
                "Aucun channel (tournament_channel_id / channel_activites), embed tournoi skip"
            );
            return;
        }
    };

    let channel_id = match u64::from_str(&channel_id_str) {
        Ok(n) => ChannelId::new(n),
        Err(_) => {
            warn!(channel_id = %channel_id_str, "channel_id invalide");
            return;
        }
    };

    let winner_mention = if !winner_id.is_empty() {
        format!("<@{}>", winner_id)
    } else {
        winner_username.to_string()
    };

    let mut top5_lines = String::new();
    for (i, entry) in top5.iter().enumerate() {
        let user_id = entry.get("user_id").and_then(|v| v.as_str()).unwrap_or("");
        let username = entry
            .get("username")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let net_gain = entry.get("net_gain").and_then(|v| v.as_i64()).unwrap_or(0);
        let mention = if !user_id.is_empty() {
            format!("<@{}>", user_id)
        } else {
            username.to_string()
        };
        let medal = match i {
            0 => "\u{1f947}",
            1 => "\u{1f948}",
            2 => "\u{1f949}",
            _ => "\u{1f539}",
        };
        top5_lines.push_str(&format!(
            "{} **{}** — {} coins nets\n",
            medal, mention, net_gain
        ));
    }
    if top5_lines.is_empty() {
        top5_lines.push_str("_Aucune activite enregistree._");
    }

    let description = format!(
        "\u{1f3c6} **Gagnant** : {}\n\u{1f4b0} **Prix** : **{}** coins ({}% de la caisse)\n\n**Top 5 de la semaine**\n{}",
        winner_mention, prize, prize_pct, top5_lines
    );

    let embed = CreateEmbed::new()
        .title("\u{1f3c6} Tournoi hebdomadaire cloture")
        .description(description)
        .footer(CreateEmbedFooter::new(format!(
            "Semaine du {}",
            week_start.chars().take(10).collect::<String>()
        )))
        .color(0xF1C40F);

    if let Err(e) = channel_id
        .send_message(&ctx.http, CreateMessage::new().embed(embed))
        .await
    {
        warn!(error = %e, channel_id = %channel_id, guild_id, "Echec post embed tournoi");
    } else {
        info!(guild_id, channel_id = %channel_id, "Embed tournoi poste");
    }
}
