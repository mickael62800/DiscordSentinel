//! Domaine guild_backup : auto-backup periodique.
//!
//! Le worker ne touche PAS Discord. Il detecte les guilds dont le composant
//! `guild-backup-bot` a `auto_backup_enabled=true` et dont l'intervalle
//! configure (`auto_backup_interval_hours`, defaut 24) est ecoule depuis la
//! derniere sauvegarde (`MAX(created_at)` dans `guild_snapshots`). Pour
//! chacune, il PUBLIE `guild_backup:capture_requested` sur `sentinel:events`
//! (meme enveloppe que l'API) ; le bot fait la capture reelle.

pub mod auto_backup;
