//! Tracker des joins récents : logique dans le core hexagonal. Le bot ne fait
//! que la lier à son type d'identifiant Discord et réexporter `JoinInfo`.

use serenity::model::id::GuildId;

pub use sentinel_core::domain::services::security::raid_analyzer::JoinInfo;

pub type RecentJoinsTracker =
    sentinel_core::domain::services::security::raid_analyzer::RecentJoinsTracker<GuildId>;
