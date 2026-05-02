//! Consumer stream : poste sur Discord les annonces planifiees publiees
//! par announcement-worker.
//!
//! Flow :
//! 1. announcement-worker tick chaque heure pile, fetch les annonces dues
//!    depuis l'API, XADD `sentinel:events` event="announcement_publish".
//! 2. Le bot (ce module) consume via event_bus::listen_stream_group,
//!    poste sur chaque channel cible (text simple ou embed riche),
//!    rapporte le resultat (channel_id, message_id, success) a l'API
//!    via POST /api/announcements/internal/runs/{run_id}/result.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serenity::all::Color;
use serenity::builder::{CreateEmbed, CreateEmbedAuthor, CreateMessage};
use serenity::model::id::ChannelId;
use serenity::prelude::*;
use tracing::{info, warn};

use sentinel_shared::api_client::BaseApiClient;
use sentinel_shared::heartbeat::ApiClientKey;

#[derive(Debug, Deserialize, Serialize, Clone)]
struct RenderedEmbed {
    title: Option<String>,
    description: String,
    color: Option<i32>,
    image_url: Option<String>,
    thumbnail_url: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct RenderedAnnouncement {
    announcement_id: String,
    run_id: String,
    guild_id: String,
    channel_ids: Vec<String>,
    content_text: String,
    embed: Option<RenderedEmbed>,
    mentions_prefix: String,
}

#[derive(Debug, Serialize)]
struct ChannelPostResult {
    channel_id: String,
    message_id: Option<String>,
    success: bool,
    error: Option<String>,
}

#[derive(Debug, Serialize)]
struct RecordRunResultBody {
    channels_posted: Vec<ChannelPostResult>,
}

/// Spawn le consumer durable. Appele une fois au `ready`.
pub fn spawn(ctx: Context) {
    tokio::spawn(async move {
        let consumer = sentinel_shared::event_bus::default_consumer_name();
        sentinel_shared::event_bus::listen_stream_group(
            "sentinel-bot-announcements".to_string(),
            consumer,
            move |payload_json| {
                let ctx = ctx.clone();
                async move { handle_event(&ctx, &payload_json).await }
            },
        )
        .await;
    });
}

async fn handle_event(ctx: &Context, payload_json: &str) {
    let envelope: serde_json::Value = match serde_json::from_str(payload_json) {
        Ok(v) => v,
        Err(_) => return,
    };
    if envelope.get("event").and_then(|v| v.as_str()) != Some("announcement_publish") {
        return;
    }
    let data = match envelope.get("data") {
        Some(d) => d.clone(),
        None => return,
    };
    let payload: RenderedAnnouncement = match serde_json::from_value(data) {
        Ok(p) => p,
        Err(e) => {
            warn!(error = %e, "announcement_publish: data invalide");
            return;
        }
    };

    info!(
        run_id = %payload.run_id,
        announcement_id = %payload.announcement_id,
        channels = payload.channel_ids.len(),
        "Posting announcement"
    );

    let mut results: Vec<ChannelPostResult> = Vec::with_capacity(payload.channel_ids.len());

    for ch_id_str in &payload.channel_ids {
        let result = post_to_channel(ctx, ch_id_str, &payload).await;
        results.push(result);
    }

    // Rapporte le resultat a l'API
    let api = {
        let data = ctx.data.read().await;
        data.get::<ApiClientKey>().cloned()
    };
    if let Some(api) = api {
        report_run_result(&api, &payload.run_id, &results).await;
    } else {
        warn!("ApiClientKey absent, impossible de reporter le resultat du run");
    }
}

async fn post_to_channel(
    ctx: &Context,
    ch_id_str: &str,
    payload: &RenderedAnnouncement,
) -> ChannelPostResult {
    let ch_id = match ch_id_str.parse::<u64>() {
        Ok(id) => ChannelId::new(id),
        Err(e) => {
            return ChannelPostResult {
                channel_id: ch_id_str.to_string(),
                message_id: None,
                success: false,
                error: Some(format!("channel_id invalide: {e}")),
            };
        }
    };

    // Construit le message : mentions_prefix + content_text (si pas d'embed)
    // ou mentions_prefix seul (si embed, le contenu va dans la description).
    let mut msg = CreateMessage::new();

    let prefix = payload.mentions_prefix.trim();
    let body = if let Some(ref embed) = payload.embed {
        let mut e = CreateEmbed::new().description(&embed.description);
        if let Some(t) = &embed.title {
            e = e.title(t.clone());
            // Petit fallback pour avoir un author/header sympa
            e = e.author(CreateEmbedAuthor::new(t));
        }
        if let Some(c) = embed.color {
            e = e.color(Color::new(c as u32));
        }
        if let Some(url) = &embed.image_url {
            if !url.is_empty() {
                e = e.image(url.clone());
            }
        }
        if let Some(url) = &embed.thumbnail_url {
            if !url.is_empty() {
                e = e.thumbnail(url.clone());
            }
        }
        msg = msg.embed(e);
        if !prefix.is_empty() {
            msg = msg.content(prefix.to_string());
        }
        Ok::<(), String>(())
    } else {
        let combined = if prefix.is_empty() {
            payload.content_text.clone()
        } else {
            format!("{}\n{}", prefix, payload.content_text)
        };
        msg = msg.content(combined);
        Ok(())
    };

    // body est juste un Result vide pour absorber les erreurs eventuelles
    let _ = body;

    match ch_id.send_message(&ctx.http, msg).await {
        Ok(message) => ChannelPostResult {
            channel_id: ch_id_str.to_string(),
            message_id: Some(message.id.to_string()),
            success: true,
            error: None,
        },
        Err(e) => {
            warn!(error = %e, channel_id = ch_id_str, "Echec envoi annonce");
            ChannelPostResult {
                channel_id: ch_id_str.to_string(),
                message_id: None,
                success: false,
                error: Some(e.to_string()),
            }
        }
    }
}

async fn report_run_result(
    api: &Arc<BaseApiClient>,
    run_id: &str,
    results: &[ChannelPostResult],
) {
    let body = RecordRunResultBody {
        channels_posted: results
            .iter()
            .map(|r| ChannelPostResult {
                channel_id: r.channel_id.clone(),
                message_id: r.message_id.clone(),
                success: r.success,
                error: r.error.clone(),
            })
            .collect(),
    };
    let path = format!("/api/announcements/internal/runs/{}/result", run_id);
    let resp: Result<serde_json::Value, String> = api.post_json(&path, &body).await;
    match resp {
        Ok(_) => info!(run_id, "Run result reported"),
        Err(e) => warn!(run_id, error = %e, "Echec report run result"),
    }
}
