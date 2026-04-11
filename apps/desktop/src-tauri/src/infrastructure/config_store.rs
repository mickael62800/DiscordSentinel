use std::fs;
use std::path::PathBuf;

use aes_gcm::aead::{Aead, KeyInit, OsRng};
use aes_gcm::{Aes256Gcm, Nonce};
use heed::types::Str;
use heed::{Database, Env, EnvOpenOptions};
use sha2::{Digest, Sha256};

use crate::domain::entities::{ApiConfig, DiscordConfig};

const DB_DIR: &str = "discord-sentinel";
const DB_NAME: &str = "config";
const TOKEN_KEY_PREFIX: &str = "bot_token:";

/// Convertit un bot_name en nom de variable .env pour le token
/// ex: "voice-bot" -> "VOICE_DISCORD_TOKEN", "audit-bot" -> "AUDIT_DISCORD_TOKEN"
fn bot_name_to_env_key(bot_name: &str) -> String {
    let prefix = bot_name
        .trim_end_matches("-bot")
        .trim_end_matches("-worker")
        .to_uppercase()
        .replace('-', "_");
    format!("{}_DISCORD_TOKEN", prefix)
}

/// Chemin du fichier .env a la racine du projet
fn env_file_path() -> Option<std::path::PathBuf> {
    // Remonter depuis l'executable jusqu'a la racine du projet
    let exe = std::env::current_exe().ok()?;
    let mut dir = exe.parent()?;
    // Remonter tant qu'on ne trouve pas .env
    for _ in 0..10 {
        let env_path = dir.join(".env");
        if env_path.exists() {
            return Some(env_path);
        }
        dir = dir.parent()?;
    }
    None
}

fn write_env_token(bot_name: &str, token: &str) -> Result<(), String> {
    let path = env_file_path().ok_or("Fichier .env introuvable")?;
    let content = fs::read_to_string(&path).map_err(|e| format!("Lecture .env: {}", e))?;
    let key = bot_name_to_env_key(bot_name);
    let new_line = format!("{}={}", key, token);

    let mut found = false;
    let mut lines: Vec<String> = content
        .lines()
        .map(|line| {
            let trimmed = line.trim();
            if trimmed.starts_with(&key) && trimmed[key.len()..].starts_with('=') {
                found = true;
                new_line.clone()
            } else {
                line.to_string()
            }
        })
        .collect();

    if !found {
        lines.push(new_line);
    }

    fs::write(&path, lines.join("\n") + "\n").map_err(|e| format!("Ecriture .env: {}", e))
}

fn remove_env_token(bot_name: &str) -> Result<(), String> {
    let path = match env_file_path() {
        Some(p) => p,
        None => return Ok(()),
    };
    let content = fs::read_to_string(&path).map_err(|e| format!("Lecture .env: {}", e))?;
    let key = bot_name_to_env_key(bot_name);

    let lines: Vec<&str> = content
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            !(trimmed.starts_with(&key) && trimmed[key.len()..].starts_with('='))
        })
        .collect();

    fs::write(&path, lines.join("\n") + "\n").map_err(|e| format!("Ecriture .env: {}", e))
}

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

    // ── Token encryption ──

    fn derive_key() -> [u8; 32] {
        let hostname = hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|_| "sentinel-fallback".into());
        let seed = format!("discord-sentinel::{}", hostname);
        let mut hasher = Sha256::new();
        hasher.update(seed.as_bytes());
        hasher.finalize().into()
    }

    fn encrypt_token(plaintext: &str) -> Result<String, String> {
        use aes_gcm::aead::rand_core::RngCore;
        let key = Self::derive_key();
        let cipher = Aes256Gcm::new_from_slice(&key)
            .map_err(|e| format!("Cipher init error: {}", e))?;
        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        let ciphertext = cipher
            .encrypt(nonce, plaintext.as_bytes())
            .map_err(|e| format!("Encryption error: {}", e))?;
        // Store as: base64(nonce + ciphertext)
        let mut combined = Vec::with_capacity(12 + ciphertext.len());
        combined.extend_from_slice(&nonce_bytes);
        combined.extend_from_slice(&ciphertext);
        use base64::Engine;
        Ok(base64::engine::general_purpose::STANDARD.encode(&combined))
    }

    pub fn save_bot_token(&self, bot_name: &str, token: &str) -> Result<(), String> {
        // 1. Sauver chiffre dans LMDB
        let encrypted = Self::encrypt_token(token)?;
        let key = format!("{}{}", TOKEN_KEY_PREFIX, bot_name);
        let mut wtxn = self.env.write_txn()
            .map_err(|e| format!("Failed to create write txn: {}", e))?;
        self.db
            .put(&mut wtxn, &key, &encrypted)
            .map_err(|e| format!("Failed to write token: {}", e))?;
        wtxn.commit()
            .map_err(|e| format!("Failed to commit: {}", e))?;
        // 2. Ecrire aussi dans le .env pour que les bots puissent le lire
        let _ = write_env_token(bot_name, token);
        Ok(())
    }


    /// Retourne (bot_name, has_token) — combine LMDB + .env
    pub fn get_all_bot_tokens(&self) -> Result<Vec<(String, bool)>, String> {
        let rtxn = self.env.read_txn()
            .map_err(|e| format!("Failed to create read txn: {}", e))?;
        let mut map = std::collections::HashMap::new();

        // LMDB tokens
        let iter = self.db.iter(&rtxn)
            .map_err(|e| format!("Failed to iterate: {}", e))?;
        for item in iter {
            let (key, _value) = item.map_err(|e| format!("Iter error: {}", e))?;
            if let Some(bot_name) = key.strip_prefix(TOKEN_KEY_PREFIX) {
                map.insert(bot_name.to_string(), true);
            }
        }

        // Completer avec les tokens du .env qui ne sont pas dans LMDB
        if let Some(path) = env_file_path() {
            if let Ok(content) = fs::read_to_string(&path) {
                for line in content.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with('#') || trimmed.is_empty() || !trimmed.contains("_DISCORD_TOKEN=") {
                        continue;
                    }
                    if let Some((env_key, value)) = trimmed.split_once('=') {
                        if value.trim().is_empty() {
                            continue;
                        }
                        // Convertir env key -> bot name: VOICE_DISCORD_TOKEN -> voice-bot
                        let bot_name = env_key
                            .trim_end_matches("_DISCORD_TOKEN")
                            .to_lowercase()
                            .replace('_', "-")
                            + "-bot";
                        map.entry(bot_name).or_insert(true);
                    }
                }
            }
        }

        Ok(map.into_iter().collect())
    }

    pub fn delete_bot_token(&self, bot_name: &str) -> Result<(), String> {
        // Supprimer du LMDB
        let key = format!("{}{}", TOKEN_KEY_PREFIX, bot_name);
        let mut wtxn = self.env.write_txn()
            .map_err(|e| format!("Failed to create write txn: {}", e))?;
        let _ = self.db.delete(&mut wtxn, &key);
        wtxn.commit()
            .map_err(|e| format!("Failed to commit: {}", e))?;
        // Supprimer aussi du .env
        let _ = remove_env_token(bot_name);
        Ok(())
    }
}
