use serenity::model::channel::{GuildChannel, Message};
use serenity::prelude::*;

use crate::audit_event;
use crate::handler::Handler;

pub async fn handle_create(ctx: &Context, channel: &GuildChannel) {
    let gid = channel.guild_id.to_string();

    Handler::log(ctx, "info", &gid, &format!(
        "Salon cree : #{} ({:?})", channel.name, channel.kind
    )).await;

    Handler::send_event(
        ctx,
        audit_event::simple(gid, "channel_create")
            .with_target(&channel.id, &channel.name)
            .with_channel(&channel.id, Some(channel.name.clone()))
            .with_details(serde_json::json!({
                "kind": format!("{:?}", channel.kind),
            })),
    )
    .await;
}

pub async fn handle_delete(
    ctx: &Context,
    channel: &GuildChannel,
    _messages: Option<Vec<Message>>,
) {
    let gid = channel.guild_id.to_string();

    Handler::log(ctx, "warn", &gid, &format!(
        "Salon supprime : #{}", channel.name
    )).await;

    Handler::send_event(
        ctx,
        audit_event::simple(gid, "channel_delete")
            .with_target(&channel.id, &channel.name)
            .with_channel(&channel.id, Some(channel.name.clone())),
    )
    .await;
}
