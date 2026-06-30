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
