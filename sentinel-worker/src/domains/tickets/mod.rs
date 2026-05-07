//! Domaine tickets : fermeture automatique des tickets inactifs
//! depuis plus de N jours (configurable par guild via
//! `bot_guild_config.inactive_close_days`, defaut 7).
//!
//! Phase 5 — la boucle existait dans le bot (`spawn_background` /
//! `close_inactive_tickets` toutes les 30 min). Migration vers worker
//! + consumer Redis : le worker UPDATE status='closed' en DB et XADD
//! un event `ticket_auto_closed`. Le bot consume, poste l'embed
//! d'avertissement dans le salon et le supprime.

pub mod close_inactive;
pub mod escalate_sla;
