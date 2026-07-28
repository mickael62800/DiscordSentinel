//! Domaine tickets : fermeture automatique des tickets inactifs
//! depuis plus de N jours (configurable par guild via
//! `bot_guild_config.inactive_close_days`, defaut 7).
//!
//! Phase 5 — la boucle existait dans le bot (`spawn_background` /
//! `close_inactive_tickets` toutes les 30 min). Migration vers worker
//! + consumer Redis : le worker UPDATE status='closed' en DB et XADD
//!   un event `ticket_auto_closed`. Le bot consume, poste l'embed
//!   d'avertissement dans le salon et le supprime.

//! SQL assumé : scans/claims ensemblistes (SELECT candidats + `UPDATE ... WHERE`
//! avec garde d'idempotence, XADD vers le bot). Les décisions métier vivent dans
//! sentinel-core (`domain::services::tickets::sla`) : `is_breached`,
//! `effective_threshold`, `DEFAULT_INACTIVE_CLOSE_DAYS`,
//! `DEFAULT_SLA_FIRST_RESPONSE_MINUTES`, `DEFAULT_SLA_ESCALATION_MINUTES`.
//! Seule exception documentée : la montée de priorité à l'escalade
//! (`urgent` jamais rétrogradé) reste dans le CASE SQL du claim atomique
//! de `escalate_sla` pour ne pas casser le fire-once multi-worker.

pub mod close_inactive;
pub mod escalate_sla;
