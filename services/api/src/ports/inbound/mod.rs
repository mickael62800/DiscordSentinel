// Bounded contexts.
pub mod ai;
pub mod audit;
pub mod casino;
pub mod community;
pub mod coude;
pub mod moderation;
pub mod system;

// Re-exports preservant l'API publique historique.

// ── ai ─────────────────────────────────────────────────────────────────────
// ── audit ──────────────────────────────────────────────────────────────────
// ── casino ─────────────────────────────────────────────────────────────────
// ── community ──────────────────────────────────────────────────────────────
// ── coude ──────────────────────────────────────────────────────────────────
// ── moderation ─────────────────────────────────────────────────────────────
// ── system ─────────────────────────────────────────────────────────────────
pub use system::manage_tickets::UpdateTicketChannelCommand;