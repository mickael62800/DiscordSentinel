pub mod cleanup;
pub mod purge;

use serenity::builder::CreateCommand;

pub fn all() -> Vec<CreateCommand> {
    vec![purge::register(), cleanup::register()]
}
