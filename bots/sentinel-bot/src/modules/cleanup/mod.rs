//! Module cleanup — /purge et /cleanup (ex cleanup-bot).

pub const MODULE_BOT_NAME: &str = "cleanup-bot";

mod api_client;
pub mod cleanup_cmd;
pub mod purge;

use serenity::all::{CommandInteraction, Context, CreateCommand};

use sentinel_shared::discord_helpers::is_module_enabled;

pub fn register_commands() -> Vec<CreateCommand> {
    vec![purge::register(), cleanup_cmd::register()]
}

pub async fn handle_command(ctx: &Context, command: &CommandInteraction) {
    if let Some(guild_id) = command.guild_id {
        if !is_module_enabled(ctx, &guild_id.to_string(), MODULE_BOT_NAME).await {
            return;
        }
    }
    match command.data.name.as_str() {
        "purge" => purge::handle(ctx, command).await,
        "cleanup" => cleanup_cmd::handle(ctx, command).await,
        _ => {}
    }
}
