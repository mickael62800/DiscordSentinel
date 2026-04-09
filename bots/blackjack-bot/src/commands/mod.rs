pub mod blackjack;
pub mod setup;

use serenity::builder::CreateCommand;

pub fn all() -> Vec<CreateCommand> {
    vec![setup::register()]
}
