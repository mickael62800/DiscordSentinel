// Value objects (immutable, sans identite). Les types pure-enum vivent
// desormais dans `crate::domain::enums` (separation enum / struct).
pub mod moderation;

// Re-exports backward-compat : tout le code existant qui fait
// `use crate::domain::enums::moderation::flag_type::FlagType;` etc. continue a marcher.
pub use crate::domain::enums::community::voice_channel_kind::VoiceChannelKind;