//! Module games — /game et mentions #Jeu (ex game-bot).

pub mod api_client;
pub mod commands;
pub mod detector;
pub mod handler;

use serenity::all::{CommandInteraction, Context, CreateCommand, Message};

pub fn register_commands() -> Vec<CreateCommand> {
    vec![commands::register()]
}

pub async fn handle_command(ctx: &Context, command: &CommandInteraction) {
    if command.data.name == "game" {
        commands::handle(ctx, command).await;
    }
}

pub async fn on_message(ctx: &Context, msg: &Message) {
    handler::on_message(ctx, msg).await;
}
