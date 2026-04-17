pub mod channel;
pub mod guild;
pub mod invite;
pub mod member;
pub mod message;
pub mod role;
pub mod thread;
pub mod voice;

use serenity::model::id::ChannelId;
use serenity::prelude::*;

pub async fn resolve_channel_name(ctx: &Context, channel_id: ChannelId) -> Option<String> {
    channel_id
        .to_channel(&ctx.http)
        .await
        .ok()
        .and_then(|c| c.guild().map(|gc| gc.name.clone()))
}
