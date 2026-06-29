//! Bootstrap : construction de l'etat applicatif (connexions infra + DI).
//!
//! Extrait de `main.rs` pour garder ce dernier concentre sur bind/serve.
//! Chaque phase de l'initialisation vit dans un sous-module dedie :
//! - `connections` : `connect_pg` / `connect_redis` (infra PostgreSQL + Redis).
//! - `inference` : `build_inference` / `build_broadcaster` (ONNX + pub/sub).
//! - `app_state` : `build_app_state` (assemble tous les repos/services).
//! - `workers` : `spawn_security_workers` (workers de fond post-bootstrap).
//!
//! Les chemins publics restent stables (`crate::bootstrap::ITEM`) via les
//! re-exports ci-dessous.

mod app_state;
mod connections;
mod inference;
mod workers;

pub use app_state::build_app_state;
pub use connections::{connect_pg, connect_redis};
pub use inference::{build_broadcaster, build_inference};
pub use workers::spawn_security_workers;
