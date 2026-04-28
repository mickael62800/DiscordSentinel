// Bounded contexts.
pub mod ai;
pub mod audit;
pub mod coude;
pub mod moderation;

// Re-exports preservant l'API publique historique.
pub use ai::inference_limiter::InferenceRateLimiter;
pub use audit::security_analyzer;
pub use coude::coude_combat_engine;
pub use moderation::channel_tension::{
    self, ChannelTensionBuffer, TensionAction, TensionEntry,
};
pub use moderation::scoring_service::{resolve_thresholds, ScoringService};
