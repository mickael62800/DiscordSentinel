//! Domaine appeal_sla : escalade des appels de sanction qui depassent
//! le SLA de premiere reponse / d'escalade (defauts respectivement 30
//! et 60 minutes — alignes sur l'ancienne migration 047 ticket-bot).
//!
//! Porte de appeal-sla-worker (Phase 2 fusion). Les constantes SLA
//! vivent dans le core (source unique, partagée avec tickets/escalate_sla).

//! SQL assumé : scan ensembliste des appels candidats + claim atomique
//! `UPDATE ... WHERE escalated_at IS NULL` (fire-once multi-worker) + XADD.
//! Les décisions vivent dans sentinel-core (`domain::services::tickets::sla`) :
//! `is_breached`, `effective_threshold` et les constantes SLA ci-dessous.

pub mod escalate_appeal_sla;

pub use sentinel_core::domain::services::tickets::sla::{
    DEFAULT_SLA_ESCALATION_MINUTES, DEFAULT_SLA_FIRST_RESPONSE_MINUTES,
};
