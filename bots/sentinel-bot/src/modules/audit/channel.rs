use serenity::model::channel::{GuildChannel, Message};
use serenity::prelude::*;

use super::audit_event;
use super::handler_impl::{Handler, WeeklyTrackerKey};
use super::weekly_report::StatField;

pub async fn handle_create(ctx: &Context, channel: &GuildChannel) {
    let gid = channel.guild_id;
    let gid_str = gid.to_string();

    Handler::log(ctx, "info", &gid_str, &format!(
        "Salon cree : #{} ({:?})", channel.name, channel.kind
    )).await;

    Handler::send_event(
        ctx,
        audit_event::simple(gid_str, "channel_create")
            .with_target(channel.id, &channel.name)
            .with_channel(channel.id, Some(channel.name.clone()))
            .with_details(serde_json::json!({
                "kind": format!("{:?}", channel.kind),
            })),
    )
    .await;

    let data = ctx.data.read().await;
    if let Some(tracker) = data.get::<WeeklyTrackerKey>() {
        tracker.increment(gid, StatField::ChannelChange);
    }
}

pub async fn handle_delete(
    ctx: &Context,
    channel: &GuildChannel,
    _messages: Option<Vec<Message>>,
) {
    let gid = channel.guild_id;
    let gid_str = gid.to_string();

    Handler::log(ctx, "warn", &gid_str, &format!(
        "Salon supprime : #{}", channel.name
    )).await;

    Handler::send_event(
        ctx,
        audit_event::simple(gid_str, "channel_delete")
            .with_target(channel.id, &channel.name)
            .with_channel(channel.id, Some(channel.name.clone())),
    )
    .await;

    let data = ctx.data.read().await;
    if let Some(tracker) = data.get::<WeeklyTrackerKey>() {
        tracker.increment(gid, StatField::ChannelChange);
    }
}
