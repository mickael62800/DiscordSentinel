// Value objects (immutable, sans identite). Les types pure-enum vivent
// desormais dans `crate::domain::enums` (separation enum / struct).
pub mod moderation;

pub use moderation::detection_flags::DetectionFlags;

// Re-exports backward-compat : tout le code existant qui fait
// `use crate::domain::value_objects::FlagType;` etc. continue a marcher.
pub use crate::domain::enums::{
    Action, CoudeClass, FlagType, ModerationActionType, ModerationGravity, TicketPriority,
    TicketStatus, VoiceChannelKind,
};
