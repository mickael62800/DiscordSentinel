// Bounded contexts re-exportes depuis sentinel-core (pur).
pub use sentinel_core::application::{ai, casino, community, coude, game, moderation};

// Bounded contexts mixtes : sentinel-core (pur) + sentinel-api (services
// avec deps infra qui n'ont pas pu migrer — manage_stats_service tient un
// redis::Client, export_service utilise sqlx::PgPool).
pub mod audit;
pub mod system;
