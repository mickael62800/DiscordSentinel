pub mod ticket;

use serenity::builder::CreateCommand;

/// Enregistre toutes les slash commands du module tickets.
pub fn all() -> Vec<CreateCommand> {
    vec![ticket::register()]
}
