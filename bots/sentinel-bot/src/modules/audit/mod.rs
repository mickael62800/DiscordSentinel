//! Module audit — logging evenements Discord (ex audit-bot).

pub mod anomaly;
pub mod audit_event;
pub mod channel;
pub mod commands;
pub mod guild;
pub mod handler_impl;
pub mod invite;
pub mod member;
pub mod message;
pub mod message_cache;
pub mod permission_diff;
pub mod role;
pub mod thread;
pub mod type_keys;
pub mod voice;
pub mod watched_users;
pub mod weekly_report;

use serenity::all::{CommandInteraction, Context, CreateCommand};

pub fn register_commands() -> Vec<CreateCommand> {
    commands::all()
}

pub async fn handle_command(ctx: &Context, command: &CommandInteraction) {
    if command.data.name == "audit" {
        commands::handle(ctx, command).await;
    }
}
