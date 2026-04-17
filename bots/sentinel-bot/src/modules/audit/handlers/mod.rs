pub mod channel;
pub mod guild;
pub mod invite;
pub mod member;
pub mod message;
pub mod role;
pub mod thread;
pub mod voice;

// Re-exports pour les enfants de handlers/ (evite les super::super::)
pub(super) use super::audit_event;
pub(super) use super::weekly_report;
pub(super) use super::watched_users;
pub(super) use super::permission_diff;
pub(super) use super::{WeeklyTrackerKey, MessageCacheKey, AnomalyDetectorKey};
pub(super) use super::{send_event, log, post_to_channel};

use serenity::model::id::ChannelId;
use serenity::prelude::*;

pub async fn resolve_channel_name(ctx: &Context, channel_id: ChannelId) -> Option<String> {
    channel_id
        .to_channel(&ctx.http)
        .await
        .ok()
        .and_then(|c| c.guild().map(|gc| gc.name.clone()))
}
