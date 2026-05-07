//! Module welcome — bienvenue/depart (ex welcome-bot).

pub const MODULE_BOT_NAME: &str = "welcome-bot";

pub mod api_client;
pub mod handler;
pub mod template;

use serenity::all::{ComponentInteraction, Context, Member};
use serenity::model::id::GuildId;

pub async fn on_member_add(ctx: &Context, member: &Member) {
    handler::on_member_add(ctx, member).await;
}

pub async fn on_member_remove(ctx: &Context, guild_id: GuildId, user: &serenity::model::user::User) {
    handler::on_member_remove(ctx, guild_id, user).await;
}

pub async fn on_component(ctx: &Context, component: &ComponentInteraction) {
    handler::on_component(ctx, component).await;
}

pub fn handles_component(custom_id: &str) -> bool {
    custom_id == handler::RULES_ACCEPT_ID
}
