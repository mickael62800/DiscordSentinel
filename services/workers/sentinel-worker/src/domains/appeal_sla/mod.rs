//! Domaine appeal_sla : escalade des appels de sanction qui depassent
//! le SLA de premiere reponse / d'escalade (defauts respectivement 30
//! et 60 minutes — alignes sur l'ancienne migration 047 ticket-bot).
//!
//! Porte de appeal-sla-worker (Phase 2 fusion). Les constantes SLA
//! restent ici parce que `escalate_appeal_sla.rs` les importe.

pub mod escalate_appeal_sla;

pub const DEFAULT_SLA_FIRST_RESPONSE_MINUTES: i64 = 30;
pub const DEFAULT_SLA_ESCALATION_MINUTES: i64 = 60;
