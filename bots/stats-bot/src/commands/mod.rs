pub mod stats;

use serenity::builder::CreateCommand;

/// Enregistre toutes les slash commands du bot.
pub fn all() -> Vec<CreateCommand> {
    vec![stats::register()]
}
