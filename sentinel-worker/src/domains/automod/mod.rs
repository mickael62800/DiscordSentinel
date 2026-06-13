//! Domaine automod : cloture des votes de moderation a echeance.

pub mod close_votes;

/// Quorum par defaut si non configure (doit matcher le defaut du
/// config_schema automod-bot, cf. migration 252).
pub const DEFAULT_VOTE_QUORUM: i32 = 3;
