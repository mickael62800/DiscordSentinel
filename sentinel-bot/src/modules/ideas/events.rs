//! Consumer stream : decisions prises depuis le web.
//!
//! L'API publie `idea_decided` sur `sentinel:events` quand le staff tranche
//! depuis la page web. Le bot reporte la decision dans le salon de l'idee et
//! previent l'auteur, exactement comme si le bouton Discord avait ete clique.

use std::collections::HashMap;

use serde::Deserialize;
use serenity::model::id::{ChannelId, UserId};
use serenity::prelude::*;
use tracing::{info, warn};

use crate::modules::ideas::constants::status_label;
use crate::modules::ideas::embed::{build_idea_embed_full, build_staff_buttons};
use crate::modules::ideas::MODULE_BOT_NAME;
use crate::shared::heartbeat::ApiClientKey;

#[derive(Debug, Deserialize)]
struct IdeaDecidedPayload {
    idea_id: String,
    guild_id: String,
    #[serde(default)]
    channel_id: Option<String>,
    title: String,
    status: String,
    author_id: String,
    #[serde(default)]
    decided_by_name: Option<String>,
    #[serde(default)]
    reason: Option<String>,
}

pub fn spawn(ctx: Context) {
    tokio::spawn(async move {
        let consumer = crate::shared::event_bus::default_consumer_name();
        crate::shared::event_bus::listen_stream_group(
            "sentinel-bot-ideas".to_string(),
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
    if envelope.get("event").and_then(|v| v.as_str()) != Some("idea_decided") {
        return;
    }
    let data = match envelope.get("data") {
        Some(d) => d.clone(),
        None => return,
    };
    let payload: IdeaDecidedPayload = match serde_json::from_value(data) {
        Ok(p) => p,
        Err(e) => {
            warn!(error = %e, "idea_decided: payload invalide");
            return;
        }
    };

    let cfg: HashMap<String, String> = {
        let data = ctx.data.read().await;
        match data.get::<ApiClientKey>() {
            Some(base) => base
                .get_guild_config_for(&payload.guild_id, MODULE_BOT_NAME)
                .await
                .unwrap_or_default(),
            None => HashMap::new(),
        }
    };

    let decided_by = payload
        .decided_by_name
        .clone()
        .unwrap_or_else(|| "Staff".to_string());

    // Le salon peut ne plus exister (idee ancienne, salon nettoye) : la
    // decision reste valable, on se contente alors du DM.
    if let Some(cid) = payload
        .channel_id
        .as_ref()
        .and_then(|c| c.parse::<u64>().ok())
    {
        let embed = build_idea_embed_full(
            &payload.idea_id,
            &payload.title,
            "",
            "autre",
            &payload.status,
            "—",
            None,
            Some((decided_by.as_str(), payload.reason.as_deref())),
            &cfg,
        );
        let mut message = serenity::builder::CreateMessage::new()
            .content("Decision prise depuis le tableau de bord :")
            .embed(embed);
        if payload.status != "realisee" {
            message = message.components(vec![build_staff_buttons()]);
        }
        if let Err(e) = ChannelId::new(cid).send_message(&ctx.http, message).await {
            warn!(error = %e, "idea_decided: publication dans le salon impossible");
        }
    }

    if let Ok(uid) = payload.author_id.parse::<u64>() {
        let motif = payload
            .reason
            .as_deref()
            .filter(|r| !r.trim().is_empty())
            .map(|r| format!("\nMotif : {r}"))
            .unwrap_or_default();
        let text = format!(
            "Ton idee « {} » est passee au statut **{}**.{motif}",
            payload.title,
            status_label(&payload.status)
        );
        if let Ok(channel) = UserId::new(uid).create_dm_channel(&ctx.http).await {
            if let Err(e) = channel.say(&ctx.http, text).await {
                tracing::debug!(error = %e, "idea_decided: DM non delivre");
            }
        }
    }

    info!(idea = %payload.idea_id, status = %payload.status, "Decision web appliquee");
}
