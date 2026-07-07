pub mod audit;

// Re-exports pour les enfants de commands/ (evite les super::super::)
pub(super) use super::api_client;

use serenity::builder::CreateCommand;

pub fn all() -> Vec<CreateCommand> {
    vec![audit::register()]
}
