use serenity::model::event::{InviteCreateEvent, InviteDeleteEvent};
use serenity::prelude::*;

use crate::audit_event;
use crate::handler::Handler;

pub async fn handle_create(ctx: &Context, data: &InviteCreateEvent) {
    let gid = match data.guild_id {
        Some(g) => g.to_string(),
        None => return,
    };

    let inviter_name = data.inviter.as_ref().map(|u| u.name.clone()).unwrap_or_else(|| "?".into());
    let inviter_id = data.inviter.as_ref().map(|u| u.id.to_string());
    let max_uses = data.max_uses;
    let max_age = data.max_age;

    Handler::log(ctx, "info", &gid, &format!(
        "Invitation creee par {} — code: {}, max uses: {}, expire: {}s",
        inviter_name, data.code, max_uses, max_age
    )).await;

    let mut evt = audit_event::simple(gid, "invite_create")
        .with_channel(&data.channel_id, None)
        .with_details(serde_json::json!({
            "code": data.code,
            "max_uses": max_uses,
            "max_age": max_age,
            "temporary": data.temporary,
        }));
    evt.actor_id = inviter_id;
    evt.actor_name = Some(inviter_name);

    Handler::send_event(ctx, evt).await;
}

pub async fn handle_delete(ctx: &Context, data: &InviteDeleteEvent) {
    let gid = match data.guild_id {
        Some(g) => g.to_string(),
        None => return,
    };

    Handler::log(ctx, "info", &gid, &format!(
        "Invitation supprimee — code: {}", data.code
    )).await;

    Handler::send_event(
        ctx,
        audit_event::simple(gid, "invite_delete")
            .with_channel(&data.channel_id, None)
            .with_details(serde_json::json!({
                "code": data.code,
            })),
    )
    .await;
}
