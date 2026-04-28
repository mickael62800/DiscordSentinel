// Bounded contexts.
pub mod ai;
pub mod audit;
pub mod coude;
pub mod moderation;

// Re-exports preservant l'API publique historique.
pub use moderation::scoring_service::ScoringService;