pub mod roles_panel;
pub mod sponsor;

use serenity::builder::CreateCommand;

pub fn all() -> Vec<CreateCommand> {
    vec![
        roles_panel::register(),
        sponsor::register(),
    ]
}
