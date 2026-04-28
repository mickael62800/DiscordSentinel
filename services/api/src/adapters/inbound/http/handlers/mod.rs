// Bounded contexts.
pub mod ai;
pub mod audit;
pub mod casino;
pub mod community;
pub mod coude;
pub mod moderation;
pub mod system;

// Re-exports preservant l'API publique historique : tout le router fait
// `handlers::xxx::fn_name` -> chaque handler reste accessible a son ancien chemin.

// ── ai ─────────────────────────────────────────────────────────────────────
// ── audit ──────────────────────────────────────────────────────────────────
// ── casino ─────────────────────────────────────────────────────────────────
// ── community ──────────────────────────────────────────────────────────────
// ── moderation (l'ancien handlers/moderation.rs est devenu moderation/actions.rs,
// glob re-export dans moderation/mod.rs preserve le path d'origine) ────────
// ── system (idem : system.rs -> info.rs, glob preserve les paths) ──────────
