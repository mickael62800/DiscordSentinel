//! Rate limiter per-user d'interactions : la logique (check-and-set atomique
//! + cleanup inline) vit dans le core hexagonal.

pub use sentinel_core::domain::services::community::interaction_cooldown::InteractionCooldown;
