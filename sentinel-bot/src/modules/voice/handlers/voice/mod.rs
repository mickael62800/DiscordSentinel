//! Handlers voice state, eclates par responsabilite.
//!
//! - `member_events` : point d'entree `handle_voice_state_update` + logique
//!   des evenements membre (join/leave/queue).
//! - `channel_lifecycle` : creation et suppression des salons temporaires
//!   (1 vocal + panneau admin dans le chat integre).

pub mod channel_lifecycle;
pub mod member_events;

pub use channel_lifecycle::handle_voice_redis_event;
pub use member_events::handle_voice_state_update;
