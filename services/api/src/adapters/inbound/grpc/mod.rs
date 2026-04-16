//! Adaptateur entrant gRPC (Phase 7A).
//!
//! Coexiste avec l'adaptateur HTTP/Axum : meme `AppState`, memes use-cases
//! domain. Le serveur tonic ecoute sur un port distinct (`GRPC_PORT`,
//! defaut 50051) et est demarre en parallele depuis `main.rs` via
//! `tokio::spawn`.
//!
//! Conversion d'erreurs : `DomainError` -> `tonic::Status` (cf. `errors`).

pub mod automod;
pub mod blackjack;
pub mod community;
pub mod coude;
pub mod export;
pub mod errors;
pub mod images;
pub mod members;
pub mod moderation;
pub mod progression;
pub mod roles;
pub mod security;
pub mod server;
pub mod stats;
pub mod tickets;
pub mod voice;
pub mod welcome;

pub use server::serve_grpc;

/// Parse un UUID depuis une string proto. Retourne `Status::invalid_argument` si invalide.
pub(crate) fn parse_uuid(s: &str) -> Result<uuid::Uuid, tonic::Status> {
    uuid::Uuid::from_str(s)
        .map_err(|_| tonic::Status::invalid_argument(format!("UUID invalide: {s}")))
}

use std::str::FromStr;
