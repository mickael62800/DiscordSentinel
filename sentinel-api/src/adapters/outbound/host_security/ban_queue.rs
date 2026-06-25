//! Adapter du port `HostBanQueue` : file-shim fichier. L'API append une ligne
//! `<ip>\t<rfc3339>\t<reason>` ; un cron host lit le fichier, applique
//! `ufw deny`/`ufw delete`, puis le vide.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

use async_trait::async_trait;

use crate::ports::outbound::system::host_ban_queue::HostBanQueue;
use sentinel_core::domain::errors::DomainError;

const BANS_PENDING_PATH: &str = "/var/lib/sentinel/bans-pending.txt";
const UNBANS_PENDING_PATH: &str = "/var/lib/sentinel/unbans-pending.txt";

#[derive(Default)]
pub struct FileBanQueue;

impl FileBanQueue {
    pub fn new() -> Self {
        Self
    }

    fn append(path: &str, ip: &str, reason: Option<&str>) -> Result<(), DomainError> {
        let parent = Path::new(path)
            .parent()
            .ok_or_else(|| DomainError::Internal(format!("chemin file invalide: {path}")))?;
        std::fs::create_dir_all(parent).map_err(|e| DomainError::Internal(format!("mkdir: {e}")))?;
        let line = format!(
            "{}\t{}\t{}\n",
            ip,
            chrono::Utc::now().to_rfc3339(),
            reason.unwrap_or("")
        );
        let mut f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|e| DomainError::Internal(format!("open {path}: {e}")))?;
        f.write_all(line.as_bytes())
            .map_err(|e| DomainError::Internal(format!("write {path}: {e}")))?;
        Ok(())
    }
}

#[async_trait]
impl HostBanQueue for FileBanQueue {
    async fn queue_ban(&self, ip: &str, reason: Option<&str>) -> Result<(), DomainError> {
        Self::append(BANS_PENDING_PATH, ip, reason)
    }

    async fn queue_unban(&self, ip: &str, reason: Option<&str>) -> Result<(), DomainError> {
        Self::append(UNBANS_PENDING_PATH, ip, reason)
    }
}
