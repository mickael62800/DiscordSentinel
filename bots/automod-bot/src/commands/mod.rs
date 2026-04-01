pub mod automod;

use serenity::builder::CreateCommand;

pub fn all() -> Vec<CreateCommand> {
    vec![automod::register()]
}
