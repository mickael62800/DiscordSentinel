//! Handlers voice state, eclates par responsabilite.
//!
//! - `member_events` : point d'entree `handle_voice_state_update` + logique
//!   des evenements membre (join/leave/queue).
//! - `channel_lifecycle` : creation et suppression des salons temporaires
//!   (categorie + vocal + panels).
//! - `channel_permissions` : grant/revoke des droits sur le panel membres.
//!
//! Les fonctions externes (`handle_voice_state_update`, `revoke_members_panel_access`)
//! sont re-exportees pour preserver `voice::handlers::voice::*`.

pub mod channel_lifecycle;
pub mod channel_permissions;
pub mod member_events;

pub use channel_permissions::revoke_members_panel_access;
pub use member_events::handle_voice_state_update;
