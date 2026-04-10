pub mod appeal;
pub mod ban;
pub mod call;
pub mod compare;
pub mod context;
pub mod evidence;
pub mod expirations;
pub mod export;
pub mod history;
pub mod mass;
pub mod modstats;
pub mod mute;
pub mod notes;
pub mod review;
pub mod template;
pub mod transcript;
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
        expirations::register(),
        compare::register(),
        modstats::register(),
        evidence::register(),
        review::register(),
        template::register(),
        transcript::register(),
        mass::register_massmute(),
        mass::register_massban(),
    ]
}
