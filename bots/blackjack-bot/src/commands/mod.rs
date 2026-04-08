pub mod blackjack;

use serenity::builder::CreateCommand;

pub fn all() -> Vec<CreateCommand> {
    vec![blackjack::register()]
}
