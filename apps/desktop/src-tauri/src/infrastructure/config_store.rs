use std::fs;
use std::path::PathBuf;

use heed::types::Str;
use heed::{Database, Env, EnvOpenOptions};

use crate::domain::entities::{ApiConfig, DiscordConfig};

const DB_DIR: &str = "discord-sentinel";
const DB_NAME: &str = "config";

pub struct ConfigStore {
    env: Env,
    db: Database<Str, Str>,
}

impl ConfigStore {
    pub fn new() -> Result<Self, String> {
        let data_dir = Self::data_dir()?;
        fs::create_dir_all(&data_dir)
            .map_err(|e| format!("Failed to create data dir: {}", e))?;

        let env = unsafe {
            EnvOpenOptions::new()
                .map_size(10 * 1024 * 1024) // 10MB
                .max_dbs(1)
                .open(&data_dir)
                .map_err(|e| format!("Failed to open LMDB: {}", e))?
        };

        let mut wtxn = env
            .write_txn()
            .map_err(|e| format!("Failed to create write txn: {}", e))?;

        let db = env
            .create_database(&mut wtxn, Some(DB_NAME))
            .map_err(|e| format!("Failed to create database: {}", e))?;

        wtxn.commit()
            .map_err(|e| format!("Failed to commit: {}", e))?;

        Ok(Self { env, db })
    }

    fn data_dir() -> Result<PathBuf, String> {
        let base = dirs::data_local_dir()
            .ok_or("Could not find local data directory")?;
        Ok(base.join(DB_DIR))
    }

    pub fn get_discord_config(&self) -> Result<Option<DiscordConfig>, String> {
        let rtxn = self.env.read_txn()
            .map_err(|e| format!("Failed to create read txn: {}", e))?;

        let client_id = self.db.get(&rtxn, "discord_client_id")
            .map_err(|e| format!("Failed to read client_id: {}", e))?;

        let client_secret = self.db.get(&rtxn, "discord_client_secret")
            .map_err(|e| format!("Failed to read client_secret: {}", e))?;

        match (client_id, client_secret) {
            (Some(id), Some(secret)) if !id.is_empty() && !secret.is_empty() => {
                Ok(Some(DiscordConfig {
                    client_id: id.to_string(),
                    client_secret: secret.to_string(),
                }))
            }
            _ => Ok(None),
        }
    }

    pub fn save_discord_config(&self, config: &DiscordConfig) -> Result<(), String> {
        let mut wtxn = self.env.write_txn()
            .map_err(|e| format!("Failed to create write txn: {}", e))?;

        self.db
            .put(&mut wtxn, "discord_client_id", &config.client_id)
            .map_err(|e| format!("Failed to write client_id: {}", e))?;

        self.db
            .put(&mut wtxn, "discord_client_secret", &config.client_secret)
            .map_err(|e| format!("Failed to write client_secret: {}", e))?;

        wtxn.commit()
            .map_err(|e| format!("Failed to commit: {}", e))
    }

    pub fn clear_discord_config(&self) -> Result<(), String> {
        let mut wtxn = self.env.write_txn()
            .map_err(|e| format!("Failed to create write txn: {}", e))?;

        let _ = self.db.delete(&mut wtxn, "discord_client_id");
        let _ = self.db.delete(&mut wtxn, "discord_client_secret");

        wtxn.commit()
            .map_err(|e| format!("Failed to commit: {}", e))
    }

    pub fn get_api_config(&self) -> Result<Option<ApiConfig>, String> {
        let rtxn = self.env.read_txn()
            .map_err(|e| format!("Failed to create read txn: {}", e))?;

        let api_url = self.db.get(&rtxn, "api_url")
            .map_err(|e| format!("Failed to read api_url: {}", e))?;

        let api_key = self.db.get(&rtxn, "api_key")
            .map_err(|e| format!("Failed to read api_key: {}", e))?;

        match (api_url, api_key) {
            (Some(url), Some(key)) if !url.is_empty() => {
                Ok(Some(ApiConfig {
                    api_url: url.to_string(),
                    api_key: key.to_string(),
                }))
            }
            (Some(url), None) if !url.is_empty() => {
                Ok(Some(ApiConfig {
                    api_url: url.to_string(),
                    api_key: String::new(),
                }))
            }
            _ => Ok(None),
        }
    }

    pub fn save_api_config(&self, config: &ApiConfig) -> Result<(), String> {
        let mut wtxn = self.env.write_txn()
            .map_err(|e| format!("Failed to create write txn: {}", e))?;

        self.db
            .put(&mut wtxn, "api_url", &config.api_url)
            .map_err(|e| format!("Failed to write api_url: {}", e))?;

        self.db
            .put(&mut wtxn, "api_key", &config.api_key)
            .map_err(|e| format!("Failed to write api_key: {}", e))?;

        wtxn.commit()
            .map_err(|e| format!("Failed to commit: {}", e))
    }
}
