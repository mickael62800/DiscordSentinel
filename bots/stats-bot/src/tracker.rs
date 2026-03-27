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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_record_messages() {
        let tracker = StatsTracker::new();
        tracker.record_message(1, 100).await;
        tracker.record_message(1, 100).await;
        tracker.record_message(1, 100).await;

        let stats = tracker.get_user_stats(1, 100).await;
        assert_eq!(stats.message_count, 3);
        assert_eq!(stats.voice_seconds, 0);
    }

    #[tokio::test]
    async fn test_different_users() {
        let tracker = StatsTracker::new();
        tracker.record_message(1, 100).await;
        tracker.record_message(1, 100).await;
        tracker.record_message(1, 200).await;

        let s1 = tracker.get_user_stats(1, 100).await;
        let s2 = tracker.get_user_stats(1, 200).await;
        assert_eq!(s1.message_count, 2);
        assert_eq!(s2.message_count, 1);
    }

    #[tokio::test]
    async fn test_different_guilds() {
        let tracker = StatsTracker::new();
        tracker.record_message(1, 100).await;
        tracker.record_message(2, 100).await;

        let s1 = tracker.get_user_stats(1, 100).await;
        let s2 = tracker.get_user_stats(2, 100).await;
        assert_eq!(s1.message_count, 1);
        assert_eq!(s2.message_count, 1);
    }

    #[tokio::test]
    async fn test_voice_leave_without_join() {
        let tracker = StatsTracker::new();
        let duration = tracker.voice_leave(1, 100).await;
        assert_eq!(duration, 0);
    }

    #[tokio::test]
    async fn test_voice_join_then_leave() {
        let tracker = StatsTracker::new();
        tracker.voice_join(1, 100).await;
        // Sleep tres court pour avoir une duree > 0
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        let duration = tracker.voice_leave(1, 100).await;
        // La duree en secondes sera 0 (< 1s) mais le code ne doit pas panic
        assert!(duration < 2);
    }

    #[tokio::test]
    async fn test_guild_stats_sorted_by_messages() {
        let tracker = StatsTracker::new();
        tracker.record_message(1, 100).await;
        tracker.record_message(1, 200).await;
        tracker.record_message(1, 200).await;
        tracker.record_message(1, 200).await;

        let guild_stats = tracker.get_guild_stats(1).await;
        assert_eq!(guild_stats.len(), 2);
        // uid 200 a 3 messages, uid 100 a 1 → 200 en premier
        assert_eq!(guild_stats[0].0, 200);
        assert_eq!(guild_stats[0].1.message_count, 3);
        assert_eq!(guild_stats[1].0, 100);
    }

    #[tokio::test]
    async fn test_unknown_user_returns_default() {
        let tracker = StatsTracker::new();
        let stats = tracker.get_user_stats(1, 999).await;
        assert_eq!(stats.message_count, 0);
        assert_eq!(stats.voice_seconds, 0);
    }
}
