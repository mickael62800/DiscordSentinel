pub mod ticket;

// Re-exports pour les enfants de commands/ (evite les super::super::)
pub(super) use super::api_client;

use serenity::builder::CreateCommand;

/// Enregistre toutes les slash commands du module tickets.
pub fn all() -> Vec<CreateCommand> {
    ticket::register()
}
