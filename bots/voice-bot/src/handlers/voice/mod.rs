//! Handlers voice state, éclatés par responsabilité.
//!
//! - `member_events` : point d'entrée `handle_voice_state_update` + logique
//!   des événements membre (join/leave/queue).
//! - `channel_lifecycle` : création et suppression des salons temporaires
//!   (catégorie + vocal + panels).
//! - `channel_permissions` : grant/revoke des droits sur le panel membres.
//!
//! Les fonctions externes (`handle_voice_state_update`, `revoke_members_panel_access`)
//! sont re-exportées pour préserver `crate::handlers::voice::*`.

pub mod channel_lifecycle;
pub mod channel_permissions;
pub mod member_events;

pub use channel_permissions::revoke_members_panel_access;
pub use member_events::handle_voice_state_update;
