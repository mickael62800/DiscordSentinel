//! Game Server (instance) — represente un container Docker piloté.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Etats possibles d'un serveur. Doit rester synchronise avec le CHECK
/// constraint en migration (188_game_portal_init.sql).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GameServerStatus {
    /// Ligne creee, container pas encore lance.
    Created,
    /// Docker start envoye, attente health.
    Starting,
    /// Container running, repond aux health checks.
    Running,
    /// Docker stop envoye, attente fin du process.
    Stopping,
    /// Container arrete proprement.
    Stopped,
    /// Crash repete ou erreur de boot, plus auto-restart.
    Error,
    /// Soft-deleted (volume + container retires). Conserve pour audit.
    Deleted,
}

impl GameServerStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Starting => "starting",
            Self::Running => "running",
            Self::Stopping => "stopping",
            Self::Stopped => "stopped",
            Self::Error => "error",
            Self::Deleted => "deleted",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "created" => Some(Self::Created),
            "starting" => Some(Self::Starting),
            "running" => Some(Self::Running),
            "stopping" => Some(Self::Stopping),
            "stopped" => Some(Self::Stopped),
            "error" => Some(Self::Error),
            "deleted" => Some(Self::Deleted),
            _ => None,
        }
    }

    /// Etats considere's "actifs" (consomment des ressources).
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Starting | Self::Running | Self::Stopping)
    }

    /// True si une transition `start` est legale depuis cet etat.
    pub fn can_start(&self) -> bool {
        matches!(self, Self::Created | Self::Stopped | Self::Error)
    }

    /// True si une transition `stop` est legale.
    pub fn can_stop(&self) -> bool {
        matches!(self, Self::Running | Self::Starting)
    }
}

/// Game Server — instance d'un template, persistee.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameServer {
    pub id: Uuid,
    pub guild_id: String,
    pub template_id: Uuid,
    pub name: String,
    pub status: GameServerStatus,
    pub container_id: Option<String>,
    pub container_name: Option<String>,
    pub host_port: Option<u16>,
    pub rcon_port: Option<u16>,
    pub rcon_password: Option<String>,
    pub volume_name: Option<String>,
    pub allocated_memory_mb: i32,
    pub owner_user_id: String,
    pub idle_shutdown_days: Option<i32>,
    pub last_active_at: Option<DateTime<Utc>>,
    pub last_player_count: i32,
    pub last_error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub stopped_at: Option<DateTime<Utc>>,
    /// Nombre de redemarrages auto consecutifs apres crash (remis a 0 a la
    /// recuperation). Borne par `MAX_RESTART_ATTEMPTS`.
    pub restart_attempts: i32,
    /// Timestamp du dernier redemarrage auto tente (pour le backoff).
    pub last_restart_at: Option<DateTime<Utc>>,
}

/// Nombre maximal de redemarrages auto consecutifs avant abandon (-> Error).
/// Borne stricte : empeche tout crash loop.
pub const MAX_RESTART_ATTEMPTS: i32 = 5;

/// Delai de backoff (secondes) avant le prochain redemarrage auto, en
/// fonction du nombre de tentatives deja effectuees. Exponentiel
/// `30 * 2^attempts`, plafonne a 1h. Pure / overflow-safe.
pub fn restart_backoff_secs(attempts: i32) -> i64 {
    const BASE: i64 = 30;
    const CAP: i64 = 3600;
    if attempts <= 0 {
        return BASE;
    }
    // 2^attempts overflow-safe : checked_shl renvoie None si shift trop grand.
    let factor = 1i64.checked_shl(attempts as u32).unwrap_or(i64::MAX);
    BASE.saturating_mul(factor).min(CAP)
}

/// Decision PURE : faut-il auto-redemarrer un serveur crashe ? `true` si on
/// est sous le plafond de tentatives ET (jamais redemarre OU le cooldown de
/// backoff est ecoule). Aucune IO -> testable unitairement.
pub fn should_auto_restart(
    attempts: i32,
    last_restart_at: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
) -> bool {
    if attempts >= MAX_RESTART_ATTEMPTS {
        return false;
    }
    match last_restart_at {
        None => true,
        Some(last) => now >= last + chrono::Duration::seconds(restart_backoff_secs(attempts)),
    }
}

impl GameServer {
    /// Nom Docker normalise pour ce serveur.
    /// Doit matcher container_name persiste en DB (cohérence reconciler).
    pub fn docker_container_name(id: Uuid) -> String {
        format!("sentinel-game-{}", id.simple())
    }

    /// Nom du volume Docker nomme pour ce serveur.
    pub fn docker_volume_name(id: Uuid) -> String {
        format!("sentinel-game-vol-{}", id.simple())
    }
}

/// Commande pour creer un serveur (input du use case).
#[derive(Debug, Clone)]
pub struct CreateGameServerCommand {
    pub guild_id: String,
    pub template_slug: String,
    pub name: String,
    pub allocated_memory_mb: Option<i32>,
    pub owner_user_id: String,
    pub initial_config: std::collections::HashMap<String, String>,
}

/// Validation regex-like du nom de serveur (alphanumerique + espaces / _ / -).
pub fn validate_server_name(name: &str) -> Result<(), String> {
    if name.is_empty() || name.len() > 64 {
        return Err("nom invalide : 1-64 caracteres".into());
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == ' ' || c == '_' || c == '-')
    {
        return Err("nom invalide : alphanumerique + espaces, _ ou - uniquement".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_start_only_from_terminal_states() {
        for st in [
            GameServerStatus::Created,
            GameServerStatus::Stopped,
            GameServerStatus::Error,
        ] {
            assert!(st.can_start(), "{st:?} devrait permettre start");
        }
        for st in [
            GameServerStatus::Starting,
            GameServerStatus::Running,
            GameServerStatus::Stopping,
            GameServerStatus::Deleted,
        ] {
            assert!(!st.can_start(), "{st:?} ne devrait pas permettre start");
        }
    }

    #[test]
    fn can_stop_only_from_active_up_states() {
        assert!(GameServerStatus::Running.can_stop());
        assert!(GameServerStatus::Starting.can_stop());
        for st in [
            GameServerStatus::Created,
            GameServerStatus::Stopped,
            GameServerStatus::Error,
            GameServerStatus::Stopping,
            GameServerStatus::Deleted,
        ] {
            assert!(!st.can_stop(), "{st:?} ne devrait pas permettre stop");
        }
    }

    #[test]
    fn backoff_is_exponential_and_capped() {
        assert_eq!(restart_backoff_secs(0), 30);
        assert_eq!(restart_backoff_secs(1), 60);
        assert_eq!(restart_backoff_secs(2), 120);
        assert_eq!(restart_backoff_secs(3), 240);
        // Plafond a 1h, jamais d'overflow meme pour de grandes valeurs.
        assert_eq!(restart_backoff_secs(20), 3600);
        assert_eq!(restart_backoff_secs(1000), 3600);
        assert_eq!(restart_backoff_secs(-1), 30);
    }

    #[test]
    fn should_restart_first_attempt_no_history() {
        let now = Utc::now();
        // Jamais redemarre -> autorise immediatement.
        assert!(should_auto_restart(0, None, now));
    }

    #[test]
    fn should_restart_when_cooldown_elapsed() {
        let now = Utc::now();
        // 2 tentatives -> backoff 120s ; dernier restart il y a 200s -> ok.
        let last = now - chrono::Duration::seconds(200);
        assert!(should_auto_restart(2, Some(last), now));
    }

    #[test]
    fn should_not_restart_within_cooldown() {
        let now = Utc::now();
        // 2 tentatives -> backoff 120s ; dernier restart il y a 30s -> non.
        let last = now - chrono::Duration::seconds(30);
        assert!(!should_auto_restart(2, Some(last), now));
    }

    #[test]
    fn should_not_restart_when_cap_reached() {
        let now = Utc::now();
        let long_ago = now - chrono::Duration::days(1);
        assert!(!should_auto_restart(
            MAX_RESTART_ATTEMPTS,
            Some(long_ago),
            now
        ));
        assert!(!should_auto_restart(MAX_RESTART_ATTEMPTS + 3, None, now));
    }

    #[test]
    fn should_restart_exactly_at_cooldown_boundary() {
        let now = Utc::now();
        // attempts=1 -> backoff 60s ; pile a la frontiere -> autorise (>=).
        let last = now - chrono::Duration::seconds(60);
        assert!(should_auto_restart(1, Some(last), now));
    }

    #[test]
    fn status_str_roundtrip() {
        for st in [
            GameServerStatus::Created,
            GameServerStatus::Starting,
            GameServerStatus::Running,
            GameServerStatus::Stopping,
            GameServerStatus::Stopped,
            GameServerStatus::Error,
            GameServerStatus::Deleted,
        ] {
            assert_eq!(GameServerStatus::from_str(st.as_str()), Some(st));
        }
    }
}
