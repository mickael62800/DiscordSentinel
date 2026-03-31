pub mod roles_panel;

use serenity::builder::CreateCommand;

pub fn all() -> Vec<CreateCommand> {
    vec![roles_panel::register()]
}
