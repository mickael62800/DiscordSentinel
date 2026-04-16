use serenity::model::channel::{GuildChannel, PartialGuildChannel};
use serenity::prelude::*;

use super::audit_event;
use super::handler_impl::Handler;

pub async fn handle_create(ctx: &Context, thread: &GuildChannel) {
    let gid = thread.guild_id.to_string();

    Handler::log(ctx, "info", &gid, &format!(
        "Fil cree : #{} (parent: {})", thread.name, thread.parent_id.map(|p| p.to_string()).unwrap_or_default()
    )).await;

    let mut evt = audit_event::simple(gid, "thread_create")
        .with_target(thread.id, &thread.name)
        .with_details(serde_json::json!({
            "kind": format!("{:?}", thread.kind),
        }));
    evt.channel_id = thread.parent_id.map(|p| p.to_string());

    Handler::send_event(ctx, evt).await;
}

pub async fn handle_delete(
    ctx: &Context,
    thread: &PartialGuildChannel,
    full_thread: Option<GuildChannel>,
) {
    let gid = thread.guild_id.to_string();
    let thread_name = full_thread
        .as_ref()
        .map(|t| t.name.clone())
        .unwrap_or_else(|| thread.id.to_string());

    Handler::log(ctx, "warn", &gid, &format!(
        "Fil supprime : #{}", thread_name
    )).await;

    let mut evt = audit_event::simple(gid, "thread_delete")
        .with_target(thread.id, &thread_name);
    evt.channel_id = full_thread.as_ref().and_then(|t| t.parent_id.map(|p| p.to_string()));

    Handler::send_event(ctx, evt).await;
}
