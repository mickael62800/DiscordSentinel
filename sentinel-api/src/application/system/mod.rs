// Service bloque dans sentinel-api : export_service utilise sqlx::PgPool directement.
pub mod export_service;

// Re-export des services purs depuis sentinel-core.
pub use sentinel_core::application::system::manage_tickets_service;
