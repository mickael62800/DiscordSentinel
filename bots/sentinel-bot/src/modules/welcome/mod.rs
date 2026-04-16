//! Module welcome — bienvenue/départ (ex welcome-bot).

pub mod api_client;
pub mod handler;
pub mod template;

use serenity::all::{Context, Member};

pub async fn on_member_add(ctx: &Context, member: &Member) {
    handler::on_member_add(ctx, member).await;
}

pub async fn on_member_remove(ctx: &Context, guild_id: serenity::model::id::GuildId, user: &serenity::model::user::User) {
    handler::on_member_remove(ctx, guild_id, user).await;
}
