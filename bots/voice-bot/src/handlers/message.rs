use serenity::model::channel::Message;
use serenity::prelude::*;
use tracing::{error, info};

use crate::api_client::{ApiClient, LogModerationActionRequest};
use crate::handler::{FloodTrackerKey, MembersToVoiceMapKey};

pub async fn handle_message(ctx: &Context, msg: &Message) {
    if msg.author.bot {
        return;
    }

    let channel_id = msg.channel_id;
    let user_id = msg.author.id;

    // Verifier si le message est dans un panel membres d'un salon temporaire
    let is_members_panel = {
        let data = ctx.data.read().await;
        data.get::<MembersToVoiceMapKey>()
            .map(|map| map.contains_key(&channel_id))
            .unwrap_or(false)
    };

    if !is_members_panel {
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
        data.get::<crate::handler::VoiceConfigKey>()
            .map(|c| c.flood_mute_duration_secs as i64)
            .unwrap_or(30)
    };
    let until = chrono::Utc::now() + chrono::Duration::seconds(mute_secs);
    let edit = serenity::builder::EditMember::new()
        .disable_communication_until(until.to_rfc3339());
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

    // Informer dans le salon
    if let Err(e) = channel_id
        .say(
            &ctx.http,
            format!("<@{user_id}> a ete **mute 30 secondes** (anti-flood)."),
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
            duration: Some(30),
        };

        if let Err(e) = api.log_moderation_action(&request).await {
            error!(error = %e, "Erreur log moderation anti-flood");
        }
    }
}
