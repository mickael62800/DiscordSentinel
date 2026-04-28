// Enums du domain regroupes par bounded context.
pub mod community;
pub mod coude;
pub mod moderation;
pub mod system;

// Re-exports plats (chaque enum etait avant accessible via
// `crate::domain::value_objects::Type` -- on garde le meme racourci ici).
pub use community::voice_channel_kind::VoiceChannelKind;
pub use coude::coude_class::CoudeClass;
pub use moderation::action::Action;
pub use moderation::flag_type::FlagType;
pub use moderation::moderation_action_type::ModerationActionType;
pub use moderation::moderation_gravity::ModerationGravity;
pub use system::ticket_priority::TicketPriority;
pub use system::ticket_status::TicketStatus;
