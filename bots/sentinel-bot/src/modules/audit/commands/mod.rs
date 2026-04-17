pub mod audit;

use serenity::builder::CreateCommand;

pub fn all() -> Vec<CreateCommand> {
    vec![audit::register()]
}
