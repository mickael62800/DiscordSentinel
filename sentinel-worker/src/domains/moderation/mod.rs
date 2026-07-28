//! Domaine moderation : nettoyage des bans temporaires expires + envoi des
//! rappels (DM programmes via DB).
//!
//! (Le systeme de points de conduite a ete supprime : modération = simple
//! historique d'infractions, plus de score.)

//! SQL assumé : ces jobs sont de purs déclencheurs temporels ensemblistes —
//! claims atomiques `UPDATE ... FOR UPDATE SKIP LOCKED` (fire-once
//! multi-worker), DELETE d'expirés, puis XADD vers le bot consommateur.
//! Aucune décision métier en Rust ici : les seuils/échéances sont des
//! timestamps déjà posés en DB à la création de la sanction (par l'API via
//! sentinel-core) ; les machines à états (`status`, `unban_status`) sont
//! encodées dans les gardes WHERE des claims.

pub mod age_unban;
pub mod cleanup_bans;
pub mod expire_temp_bans;
pub mod send_reminders;

pub mod sursis_expire;
