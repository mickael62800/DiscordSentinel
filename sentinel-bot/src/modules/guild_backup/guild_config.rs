//! Configuration per-guild du composant `guild-backup-bot`.
//!
//! Lue depuis l'API (table `bot_guild_config`, stockee sous le nom de bot
//! `guild-backup-bot`). Fournit des defauts raisonnables si non configuree.
//! Consommee surtout par le chemin EVENT (pilotage web) pour decider si le
//! composant est actif et appliquer le quota de snapshots.

#![allow(dead_code)]

use std::collections::HashMap;

use crate::shared::api_client::BaseApiClient;

/// Nom sous lequel la config du composant est stockee cote API.
pub const MODULE_BOT_NAME: &str = "guild-backup-bot";

/// Configuration du composant guild-backup pour une guild.
pub struct Config {
    raw: HashMap<String, String>,
}

impl Config {
    /// Charge la config depuis l'API. Config vide (defauts) si l'appel echoue.
    pub async fn load(api: &BaseApiClient, guild_id: &str) -> Self {
        let raw = match api.get_guild_config_for(guild_id, MODULE_BOT_NAME).await {
            Ok(cfg) => cfg,
            Err(e) => {
                tracing::warn!(error = %e, guild_id = %guild_id, "guild_backup: echec get_guild_config");
                HashMap::new()
            }
        };
        Self { raw }
    }

    /// Composant active pour cette guild ? (defaut: false, fail-closed)
    pub fn enabled(&self) -> bool {
        BaseApiClient::config_bool(&self.raw, "enabled", false)
    }

    /// Quota de snapshots conserves (defaut: 10). Les plus anciens au-dela sont
    /// elagues. NB: l'API applique deja son propre quota (20 en dur cote
    /// service) — le plus petit des deux s'applique de fait.
    pub fn snapshot_quota(&self) -> u64 {
        BaseApiClient::config_u64(&self.raw, "snapshot_quota", 10)
    }

    /// Backups automatiques actives ? (defaut: false)
    pub fn auto_backup_enabled(&self) -> bool {
        BaseApiClient::config_bool(&self.raw, "auto_backup_enabled", false)
    }

    /// Intervalle (heures) entre deux backups automatiques (defaut: 24).
    pub fn auto_backup_interval_hours(&self) -> u64 {
        BaseApiClient::config_u64(&self.raw, "auto_backup_interval_hours", 24)
    }

    /// Roles autorises a declencher une restauration (CSV de role_id). Vide =>
    /// owner uniquement.
    pub fn restore_role_ids(&self) -> Vec<String> {
        let raw = BaseApiClient::config_or(&self.raw, "restore_role_ids", "");
        crate::shared::parsers::split_csv(&raw)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(entries: &[(&str, &str)]) -> Config {
        Config {
            raw: entries
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        }
    }

    #[test]
    fn defaults_when_empty() {
        let c = cfg(&[]);
        assert!(c.enabled());
        assert_eq!(c.snapshot_quota(), 10);
        assert!(!c.auto_backup_enabled());
        assert_eq!(c.auto_backup_interval_hours(), 24);
        assert!(c.restore_role_ids().is_empty());
    }

    #[test]
    fn parses_overrides() {
        let c = cfg(&[
            ("enabled", "false"),
            ("snapshot_quota", "3"),
            ("restore_role_ids", " 111 , 222 ,,333 "),
        ]);
        assert!(!c.enabled());
        assert_eq!(c.snapshot_quota(), 3);
        assert_eq!(c.restore_role_ids(), vec!["111", "222", "333"]);
    }
}
