use serenity::model::channel::Message;
use serenity::model::event::MessageUpdateEvent;
use serenity::model::id::{ChannelId, GuildId, MessageId};
use serenity::prelude::*;

use crate::audit_event;
use crate::handler::Handler;

pub async fn handle_delete(
    ctx: &Context,
    channel_id: ChannelId,
    message_id: MessageId,
    guild_id: Option<GuildId>,
) {
    let gid = match guild_id {
        Some(g) => g.to_string(),
        None => return,
    };

    let channel_name = super::resolve_channel_name(ctx, channel_id).await;
    let chan_label = channel_name.as_deref().unwrap_or("?");

    Handler::log(ctx, "warn", &gid, &format!("Message {} supprime dans #{}", message_id, chan_label)).await;

    let mut evt = audit_event::simple(gid, "message_delete")
        .with_channel(channel_id, channel_name);
    evt.target_id = Some(message_id.to_string());

    Handler::send_event(ctx, evt).await;
}

pub async fn handle_update(
    ctx: &Context,
    old: Option<Message>,
    _new: Option<Message>,
    event: MessageUpdateEvent,
) {
    let gid = match event.guild_id {
        Some(g) => g.to_string(),
        None => return,
    };

    let author_id = event.author.as_ref().map(|a| a.id.to_string());
    let author_name = event.author.as_ref().map(|a| a.name.clone());
    let new_content = event.content.clone().unwrap_or_default();
    let old_content = old.as_ref().map(|m| m.content.clone()).unwrap_or_default();

    let name = author_name.as_deref().unwrap_or("?");
    Handler::log(ctx, "info", &gid, &format!(
        "{} a modifie un message — avant: \"{}\" | apres: \"{}\"",
        name,
        if old_content.is_empty() { "(inconnu)" } else { &old_content },
        new_content
    )).await;

    let mut evt = audit_event::simple(gid, "message_edit")
        .with_channel(event.channel_id, None)
        .with_details(serde_json::json!({
            "old_content": old_content,
            "new_content": new_content,
        }));
    evt.target_id = Some(event.id.to_string());
    evt.actor_id = author_id;
    evt.actor_name = author_name;

    Handler::send_event(ctx, evt).await;
}

pub async fn handle_delete_bulk(
    ctx: &Context,
    channel_id: ChannelId,
    multiple_deleted: Vec<MessageId>,
    guild_id: Option<GuildId>,
) {
    let gid = match guild_id {
        Some(g) => g.to_string(),
        None => return,
    };

    let count = multiple_deleted.len();
    let channel_name = super::resolve_channel_name(ctx, channel_id).await;
    let chan_label = channel_name.as_deref().unwrap_or("?");

    Handler::log(ctx, "error", &gid, &format!(
        "Purge : {} messages supprimes dans #{}", count, chan_label
    )).await;

    Handler::send_event(
        ctx,
        audit_event::simple(gid, "message_delete_bulk")
            .with_channel(channel_id, channel_name)
            .with_details(serde_json::json!({
                "count": count,
                "message_ids": multiple_deleted.iter().map(|id| id.to_string()).collect::<Vec<_>>(),
            })),
    )
    .await;
}
