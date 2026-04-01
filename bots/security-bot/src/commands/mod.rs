pub mod security;

use serenity::builder::CreateCommand;

pub fn all() -> Vec<CreateCommand> {
    vec![security::register()]
}
