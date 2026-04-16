//! Module audit — logging evenements Discord (ex audit-bot).
//! TODO: migration des 17 sous-modules en cours.

use serenity::all::{CommandInteraction, Context, CreateCommand};

pub fn register_commands() -> Vec<CreateCommand> {
    // TODO: audit commands
    vec![]
}

pub async fn handle_command(_ctx: &Context, _command: &CommandInteraction) {
    // TODO
}
