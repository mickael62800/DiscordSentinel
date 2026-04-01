pub mod appeal;
pub mod ban;
pub mod call;
pub mod context;
pub mod export;
pub mod history;
pub mod mass;
pub mod mute;
pub mod notes;
pub mod warn;

use serenity::builder::CreateCommand;

pub fn all() -> Vec<CreateCommand> {
    vec![
        warn::register(),
        mute::register(),
        mute::register_unmute(),
        ban::register(),
        ban::register_unban(),
        history::register(),
        notes::register(),
        call::register(),
        context::register(),
        appeal::register(),
        export::register(),
        mass::register_massmute(),
        mass::register_massban(),
    ]
}
