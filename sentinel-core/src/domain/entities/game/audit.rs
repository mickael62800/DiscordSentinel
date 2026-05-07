//! Audit log dedie au game portal (qui a fait quoi sur quel serveur).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Actions recensees. Etendre selon les besoins futurs (backup, restore...).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GameAuditAction {
    Create,
    Start,
    Stop,
    Restart,
    Delete,
    ConfigUpdate,
    CommandRcon,
    IdleShutdown,
    CrashDetected,
    AutoRestart,
    BackupCreate,
    BackupRestore,
}

impl GameAuditAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Create => "create",
            Self::Start => "start",
            Self::Stop => "stop",
            Self::Restart => "restart",
            Self::Delete => "delete",
            Self::ConfigUpdate => "config_update",
            Self::CommandRcon => "command_rcon",
            Self::IdleShutdown => "idle_shutdown",
            Self::CrashDetected => "crash_detected",
            Self::AutoRestart => "auto_restart",
            Self::BackupCreate => "backup_create",
            Self::BackupRestore => "backup_restore",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameAuditEntry {
    pub id: Uuid,
    pub server_id: Option<Uuid>,
    pub guild_id: String,
    pub actor_user_id: Option<String>,
    pub action: String,
    pub details: serde_json::Value,
    pub created_at: DateTime<Utc>,
}
