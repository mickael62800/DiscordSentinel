use serenity::model::voice::VoiceState;
use serenity::prelude::*;

use crate::audit_event;
use crate::handler::{Handler, WeeklyTrackerKey};
use crate::weekly_report::StatField;

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

    let voice_msg = match event_type {
        "voice_join" => format!("{} a rejoint le salon vocal {}", user_name, new_channel.unwrap()),
        "voice_leave" => format!("{} a quitte le salon vocal {}", user_name, old_channel.unwrap()),
        "voice_move" => format!("{} a change de salon vocal {} -> {}", user_name, old_channel.unwrap(), new_channel.unwrap()),
        _ => String::new(),
    };
    Handler::log(ctx, "info", &gid, &voice_msg).await;

    let mut evt = audit_event::simple(gid, event_type)
        .with_actor(&user_id, &user_name)
        .with_details(serde_json::json!({
            "from_channel": old_channel.map(|c| c.to_string()),
            "to_channel": new_channel.map(|c| c.to_string()),
        }));
    evt.channel_id = new_channel.map(|c| c.to_string());

    Handler::send_event(ctx, evt).await;

    if let Some(guild_id) = new.guild_id {
        let data = ctx.data.read().await;
        if let Some(tracker) = data.get::<WeeklyTrackerKey>() {
            tracker.increment(guild_id, StatField::VoiceEvent);
        }
    }
}
