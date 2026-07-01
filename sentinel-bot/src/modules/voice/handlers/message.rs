use serenity::model::channel::Message;
use serenity::prelude::*;
use tracing::{error, info};

use super::api_client::{ApiClient, LogModerationActionRequest};
use super::{FloodTrackerKey, VoiceOwnerMapKey};

/// Formate une duree en secondes vers un texte FR lisible
/// (ex: 30 -> "30 secondes", 90 -> "1 minute 30 secondes").
fn format_fr_duration(secs: i64) -> String {
    if secs < 60 {
        return format!("{secs} seconde{}", if secs.abs() > 1 { "s" } else { "" });
    }
    let mins = secs / 60;
    let rem = secs % 60;
    let mut out = format!("{mins} minute{}", if mins > 1 { "s" } else { "" });
    if rem > 0 {
        out.push_str(&format!(" {rem} seconde{}", if rem > 1 { "s" } else { "" }));
    }
    out
}

pub async fn handle_message(ctx: &Context, msg: &Message) {
    if msg.author.bot {
        return;
    }

    let channel_id = msg.channel_id;
    let user_id = msg.author.id;

    // Verifier si le message est dans le chat integre d'un vocal temporaire.
    let is_temp_voice_chat = {
        let data = ctx.data.read().await;
        data.get::<VoiceOwnerMapKey>()
            .map(|map| map.contains_key(&channel_id))
            .unwrap_or(false)
    };

    if !is_temp_voice_chat {
        return;
    }

    // Verifier le flood
    let is_flood = {
        let data = ctx.data.read().await;
        data.get::<FloodTrackerKey>()
            .map(|tracker| tracker.record_message(channel_id, user_id))
            .unwrap_or(false)
    };

    if !is_flood {
        return;
    }

    let Some(guild_id) = msg.guild_id else {
        return;
    };

    // Mute configurable (default 30s, lu depuis VoiceConfig).
    let mute_secs = {
        let data = ctx.data.read().await;
        data.get::<super::VoiceConfigKey>()
            .map(|c| c.flood_mute_duration_secs as i64)
            .unwrap_or(30)
    };
    let until = chrono::Utc::now() + chrono::Duration::seconds(mute_secs);
    let edit = serenity::builder::EditMember::new().disable_communication_until(until.to_rfc3339());
    match guild_id.edit_member(&ctx.http, user_id, edit).await {
        Ok(_) => info!(user = %user_id, mute_secs, "Anti-flood: mute"),
        Err(why) => error!(error = %why, "Erreur mute anti-flood"),
    }

    // Clear le compteur
    {
        let data = ctx.data.read().await;
        if let Some(tracker) = data.get::<FloodTrackerKey>() {
            tracker.clear(channel_id, user_id);
        }
    }

    // Informer dans le salon (duree configuree, formatee en FR).
    let mute_human = format_fr_duration(mute_secs);
    if let Err(e) = channel_id
        .say(
            &ctx.http,
            format!("<@{user_id}> a ete **mute {mute_human}** (anti-flood)."),
        )
        .await
    {
        tracing::warn!(error = %e, "failed to send anti-flood notification");
    }

    // Logger l'action via l'API
    let data = ctx.data.read().await;
    if let Some(api) = ApiClient::from_data(&data) {
        let request = LogModerationActionRequest {
            guild_id: guild_id.get().to_string(),
            channel_id: channel_id.get().to_string(),
            moderator_id: "voice-bot".to_string(),
            moderator_name: "Voice Bot (anti-flood)".to_string(),
            target_id: user_id.get().to_string(),
            target_name: msg.author.name.clone(),
            action_type: "mute".to_string(),
            reason: "Anti-flood: 5+ messages en 5 secondes".to_string(),
            duration: Some(mute_secs),
        };

        if let Err(e) = api.log_moderation_action(&request).await {
            error!(error = %e, "Erreur log moderation anti-flood");
        }
    }
}
