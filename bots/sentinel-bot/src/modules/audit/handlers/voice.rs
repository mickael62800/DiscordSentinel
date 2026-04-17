use serenity::model::voice::VoiceState;
use serenity::prelude::*;

use super::super::audit_event;
use super::super::{WeeklyTrackerKey, watched_users};
use super::super::{send_event, log};
use super::super::weekly_report::StatField;

pub async fn handle_state_update(ctx: &Context, old: Option<VoiceState>, new: &VoiceState) {
    let gid = match new.guild_id {
        Some(g) => g.to_string(),
        None => return,
    };

    let user_id = new.user_id.to_string();
    let user_name = new
        .member
        .as_ref()
        .map(|m| m.user.name.clone())
        .unwrap_or_default();

    let old_channel = old.as_ref().and_then(|o| o.channel_id);
    let new_channel = new.channel_id;

    let event_type = match (old_channel, new_channel) {
        (None, Some(_)) => "voice_join",
        (Some(_), None) => "voice_leave",
        (Some(a), Some(b)) if a != b => "voice_move",
        _ => return,
    };

    let voice_msg = match (event_type, old_channel, new_channel) {
        ("voice_join", _, Some(ch)) => format!("{} a rejoint le salon vocal {}", user_name, ch),
        ("voice_leave", Some(ch), _) => format!("{} a quitte le salon vocal {}", user_name, ch),
        ("voice_move", Some(old), Some(new)) => format!("{} a change de salon vocal {} -> {}", user_name, old, new),
        _ => String::new(),
    };
    log(ctx, "info", &gid, &voice_msg).await;

    let mut evt = audit_event::simple(gid.clone(), event_type)
        .with_actor(&user_id, &user_name)
        .with_details(serde_json::json!({
            "from_channel": old_channel.map(|c| c.to_string()),
            "to_channel": new_channel.map(|c| c.to_string()),
        }));
    evt.channel_id = new_channel.map(|c| c.to_string());

    send_event(ctx, evt).await;

    // Surveillance
    let channel_str = new_channel.or(old_channel).map(|c| c.to_string());
    watched_users::track_activity(
        ctx, &gid, &user_id, event_type,
        channel_str.as_deref(), None,
        Some(&voice_msg),
        serde_json::json!({"from": old_channel.map(|c| c.to_string()), "to": new_channel.map(|c| c.to_string())}),
    ).await;

    if let Some(guild_id) = new.guild_id {
        let data = ctx.data.read().await;
        if let Some(tracker) = data.get::<WeeklyTrackerKey>() {
            tracker.increment(guild_id, StatField::VoiceEvent);
        }
    }
}
