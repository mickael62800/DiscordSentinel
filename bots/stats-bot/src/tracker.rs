use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;
use tokio::sync::RwLock;

/// Clé unique (guild_id, user_id).
type UserKey = (u64, u64);

/// Statistiques en mémoire pour un utilisateur.
#[derive(Debug, Clone, Default)]
pub struct UserStats {
    pub message_count: u64,
    pub voice_seconds: u64,
}

/// Suivi de l'entrée en vocal d'un utilisateur (timestamp join).
#[derive(Debug, Clone)]
struct VoiceSession {
    joined_at: i64,
}

/// Tracker in-memory pour les messages et le temps vocal.
#[derive(Debug, Clone)]
pub struct StatsTracker {
    stats: Arc<RwLock<HashMap<UserKey, UserStats>>>,
    voice_sessions: Arc<RwLock<HashMap<UserKey, VoiceSession>>>,
}

impl StatsTracker {
    pub fn new() -> Self {
        Self {
            stats: Arc::new(RwLock::new(HashMap::new())),
            voice_sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Incrémente le compteur de messages d'un utilisateur.
    pub async fn record_message(&self, guild_id: u64, user_id: u64) {
        let mut stats = self.stats.write().await;
        stats
            .entry((guild_id, user_id))
            .or_default()
            .message_count += 1;
    }

    /// Enregistre l'entrée d'un utilisateur dans un salon vocal.
    pub async fn voice_join(&self, guild_id: u64, user_id: u64) {
        let mut sessions = self.voice_sessions.write().await;
        sessions.insert(
            (guild_id, user_id),
            VoiceSession {
                joined_at: Utc::now().timestamp(),
            },
        );
    }

    /// Enregistre la sortie d'un utilisateur du salon vocal, cumule le temps et retourne les secondes.
    pub async fn voice_leave(&self, guild_id: u64, user_id: u64) -> u64 {
        let mut sessions = self.voice_sessions.write().await;
        if let Some(session) = sessions.remove(&(guild_id, user_id)) {
            let duration = (Utc::now().timestamp() - session.joined_at).max(0) as u64;
            let mut stats = self.stats.write().await;
            stats
                .entry((guild_id, user_id))
                .or_default()
                .voice_seconds += duration;
            duration
        } else {
            0
        }
    }

    /// Récupère les stats d'un utilisateur (inclut le temps vocal en cours).
    pub async fn get_user_stats(&self, guild_id: u64, user_id: u64) -> UserStats {
        let stats = self.stats.read().await;
        let mut result = stats
            .get(&(guild_id, user_id))
            .cloned()
            .unwrap_or_default();

        // Ajouter le temps de la session vocale en cours
        let sessions = self.voice_sessions.read().await;
        if let Some(session) = sessions.get(&(guild_id, user_id)) {
            let ongoing = (Utc::now().timestamp() - session.joined_at).max(0) as u64;
            result.voice_seconds += ongoing;
        }

        result
    }

    /// Récupère les stats de tous les utilisateurs d'un serveur.
    pub async fn get_guild_stats(&self, guild_id: u64) -> Vec<(u64, UserStats)> {
        let stats = self.stats.read().await;
        let sessions = self.voice_sessions.read().await;
        let now = Utc::now().timestamp();

        let mut result: HashMap<u64, UserStats> = HashMap::new();

        for (&(gid, uid), s) in stats.iter() {
            if gid == guild_id {
                result.insert(uid, s.clone());
            }
        }

        // Ajouter les sessions vocales en cours
        for (&(gid, uid), session) in sessions.iter() {
            if gid == guild_id {
                let ongoing = (now - session.joined_at).max(0) as u64;
                result.entry(uid).or_default().voice_seconds += ongoing;
            }
        }

        let mut entries: Vec<_> = result.into_iter().collect();
        entries.sort_by(|a, b| b.1.message_count.cmp(&a.1.message_count));
        entries
    }
}
