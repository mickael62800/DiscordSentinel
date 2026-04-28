// Bounded contexts (regroupent les ~80 entites par domaine fonctionnel).
pub mod ai;
pub mod audit;
pub mod casino;
pub mod community;
pub mod coude;
pub mod moderation;
pub mod system;

// Re-export pour preserver l'API publique historique : tout le reste du code
// continue a faire `use crate::domain::entities::Type` ou
// `use crate::domain::entities::system::analytics::Foo` sans changer un import.
// PR2 (cosmetique) renommera les fichiers `coude_*.rs` -> `*.rs` dans coude/.

// ── ai ─────────────────────────────────────────────────────────────────────
// ── audit ──────────────────────────────────────────────────────────────────
// ── casino ─────────────────────────────────────────────────────────────────
// ── community ──────────────────────────────────────────────────────────────
// ── coude (le jeu) ─────────────────────────────────────────────────────────
// ── moderation ─────────────────────────────────────────────────────────────
// ── system (transverses) ───────────────────────────────────────────────────
// `analytics` reste accessible aussi en tant que sous-module qualifie
// (`crate::domain::entities::system::analytics::Foo`) :
pub use system::ticket::TicketMessage;