pub mod image;

use serenity::builder::CreateCommand;

pub fn all() -> Vec<CreateCommand> {
    vec![image::register()]
}
