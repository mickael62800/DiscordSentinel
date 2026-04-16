//! Module roles — panels de rôles + sync (ex roles-bot).

pub mod api_client;
pub mod handler;
pub mod roles_panel;

use serenity::all::{CommandInteraction, Context, CreateCommand};

pub fn register_commands() -> Vec<CreateCommand> {
    vec![roles_panel::register()]
}

pub async fn handle_command(ctx: &Context, command: &CommandInteraction) {
    if command.data.name == "roles-panel" {
        roles_panel::handle(ctx, command).await;
    }
}
