//! Consumer stream : poste un message TEXTE dans un salon.
//!
//! Le pendant depouille du module `embeds` : pas de carte, pas de titre, pas
//! de couleur — le contenu markdown tel qu'il a ete ecrit. C'est ce qu'on veut
//! quand le message doit ressembler a celui d'un membre plutot qu'a une
//! notification de service.
//!
//! Flow :
//! 1. Le back-office appelle `POST /api/messages/{guild_id}/{channel_id}`.
//! 2. L'API publie `XADD sentinel:events event="message_send"`.
//! 3. Ce module consomme et poste.
//!
//! Pourquoi passer par le bot plutot que laisser l'API appeler Discord : le
//! message doit venir du bot (son identite, son avatar, ses permissions dans
//! le salon), et le bot est deja le seul point qui encaisse les rate-limits
//! Discord. Deux emetteurs, ce sont deux compteurs qui s'ignorent.

use serde::{Deserialize, Serialize};
use serenity::builder::{CreateAllowedMentions, CreateMessage};
use serenity::model::id::ChannelId;
use serenity::prelude::*;
use tracing::{info, warn};

/// Limite Discord pour le contenu d'un message. L'API valide deja, mais elle
/// n'est pas seule a pouvoir alimenter le stream : mieux vaut refuser ici que
/// d'envoyer un message que Discord rejettera.
const MAX_CONTENT: usize = 2000;

#[derive(Debug, Deserialize, Serialize, Clone)]
struct MessageSend {
    guild_id: String,
    channel_id: String,
    content: String,
}

/// Spawn le consumer durable. Appele une fois au `ready`.
pub fn spawn(ctx: Context) {
    tokio::spawn(async move {
        let consumer = crate::shared::event_bus::default_consumer_name();
        crate::shared::event_bus::listen_stream_group(
            "sentinel-bot-messages".to_string(),
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
    let Ok(envelope) = serde_json::from_str::<serde_json::Value>(payload_json) else {
        return;
    };
    if envelope.get("event").and_then(|v| v.as_str()) != Some("message_send") {
        return;
    }
    let Some(data) = envelope.get("data").cloned() else {
        return;
    };
    let payload: MessageSend = match serde_json::from_value(data) {
        Ok(p) => p,
        Err(e) => {
            warn!(error = %e, "message_send: payload invalide");
            return;
        }
    };

    if payload.content.trim().is_empty() || payload.content.chars().count() > MAX_CONTENT {
        warn!(
            channel = %payload.channel_id,
            "message_send: contenu vide ou trop long, ignore"
        );
        return;
    }

    let Ok(channel_id) = payload.channel_id.parse::<u64>() else {
        warn!(channel = %payload.channel_id, "message_send: channel_id invalide");
        return;
    };

    // Mentions laissees libres : l'outil est reserve au back-office, et un
    // message d'annonce qui ne peut pas ping @everyone ne sert a rien. Le
    // garde-fou est donc l'acces a la page, pas le contenu.
    let message = CreateMessage::new()
        .content(&payload.content)
        .allowed_mentions(
            CreateAllowedMentions::new()
                .everyone(true)
                .all_roles(true)
                .all_users(true),
        );

    match ChannelId::new(channel_id).send_message(&ctx.http, message).await {
        Ok(m) => info!(
            guild = %payload.guild_id,
            channel = %payload.channel_id,
            message = %m.id,
            "message texte poste"
        ),
        Err(e) => warn!(
            channel = %payload.channel_id,
            error = %e,
            "echec du post de message texte"
        ),
    }
}
