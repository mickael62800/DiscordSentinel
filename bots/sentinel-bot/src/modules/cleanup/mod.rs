//! Module cleanup — /purge et /cleanup (ex cleanup-bot).

mod api_client;
pub mod cleanup_cmd;
pub mod purge;

use serenity::all::{CommandInteraction, Context, CreateCommand};

pub fn register_commands() -> Vec<CreateCommand> {
    vec![purge::register(), cleanup_cmd::register()]
}

pub async fn handle_command(ctx: &Context, command: &CommandInteraction) {
    match command.data.name.as_str() {
        "purge" => purge::handle(ctx, command).await,
        "cleanup" => cleanup_cmd::handle(ctx, command).await,
        _ => {}
    }
}
