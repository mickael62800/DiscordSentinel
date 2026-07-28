//! Cache des messages pour l'audit : la logique (LRU par guild, éviction par
//! snowflake croissant, garde 2x) vit dans le core hexagonal. Le bot ne fait
//! que la lier à ses types Discord.

use serenity::model::id::{GuildId, MessageId};

pub use sentinel_core::domain::services::audit::message_cache::CachedMessage;

pub type MessageCache =
    sentinel_core::domain::services::audit::message_cache::MessageCache<GuildId, MessageId>;
