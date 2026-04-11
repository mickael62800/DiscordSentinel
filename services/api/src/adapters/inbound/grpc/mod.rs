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
pub mod coude;
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

pub use server::serve_grpc;
