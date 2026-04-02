use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;
use tokio::sync::RwLock;

/// Cle unique (guild_id, user_id).
type UserKey = (u64, u64);

/// Statistiques en memoire pour un utilisateur.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct UserStats {
    pub message_count: u64,
    pub voice_seconds: u64,
}

/// Suivi de l'entree en vocal d'un utilisateur (timestamp join).
#[derive(Debug, Clone)]
struct VoiceSession {
    joined_at: i64,
}

/// Tracker in-memory pour les messages et le temps vocal.
/// Utilise comme cache local / fallback quand l'API backend est indisponible.
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

    /// Incremente le compteur de messages d'un utilisateur.
    pub async fn record_message(&self, guild_id: u64, user_id: u64) {
        let mut stats = self.stats.write().await;
        stats
            .entry((guild_id, user_id))
            .or_default()
            .message_count += 1;
    }

    /// Enregistre l'entree d'un utilisateur dans un salon vocal.
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

    /// Verifie si un utilisateur est actuellement en session vocale.
    pub async fn is_in_voice(&self, guild_id: u64, user_id: u64) -> bool {
        let sessions = self.voice_sessions.read().await;
        sessions.contains_key(&(guild_id, user_id))
    }

    /// Retourne le nombre d'utilisateurs suivis pour un serveur.
    pub async fn tracked_users_count(&self, guild_id: u64) -> usize {
        let stats = self.stats.read().await;
        stats.keys().filter(|(gid, _)| *gid == guild_id).count()
    }

    /// Retourne le nombre de sessions vocales actives pour un serveur.
    pub async fn active_voice_sessions(&self, guild_id: u64) -> usize {
        let sessions = self.voice_sessions.read().await;
        sessions.keys().filter(|(gid, _)| *gid == guild_id).count()
    }

    /// Recupere les stats d'un utilisateur (inclut le temps vocal en cours).
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

    /// Recupere les stats de tous les utilisateurs d'un serveur, triees par nombre de messages.
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

    // ── Messages ──

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
    async fn test_unknown_user_returns_default() {
        let tracker = StatsTracker::new();
        let stats = tracker.get_user_stats(1, 999).await;
        assert_eq!(stats, UserStats::default());
    }

    #[tokio::test]
    async fn test_many_messages_same_user() {
        let tracker = StatsTracker::new();
        for _ in 0..1000 {
            tracker.record_message(1, 42).await;
        }
        let stats = tracker.get_user_stats(1, 42).await;
        assert_eq!(stats.message_count, 1000);
    }

    // ── Voice ──

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
    async fn test_voice_double_join_overwrites() {
        let tracker = StatsTracker::new();
        tracker.voice_join(1, 100).await;
        // Un deuxieme join ecrase le premier (re-join sans leave)
        tracker.voice_join(1, 100).await;
        let duration = tracker.voice_leave(1, 100).await;
        assert!(duration < 2);
    }

    #[tokio::test]
    async fn test_voice_double_leave_returns_zero() {
        let tracker = StatsTracker::new();
        tracker.voice_join(1, 100).await;
        let d1 = tracker.voice_leave(1, 100).await;
        let d2 = tracker.voice_leave(1, 100).await;
        // Le premier leave retourne la duree, le second retourne 0
        assert!(d1 < 2);
        assert_eq!(d2, 0);
    }

    #[tokio::test]
    async fn test_voice_cumulates_across_sessions() {
        let tracker = StatsTracker::new();

        // Session 1
        tracker.voice_join(1, 100).await;
        tracker.voice_leave(1, 100).await;

        // Session 2
        tracker.voice_join(1, 100).await;
        tracker.voice_leave(1, 100).await;

        // Les deux sessions sont cumulees
        let stats = tracker.get_user_stats(1, 100).await;
        // Le total devrait etre la somme des deux (probablement 0+0=0 car < 1s)
        assert!(stats.voice_seconds < 4);
    }

    #[tokio::test]
    async fn test_is_in_voice() {
        let tracker = StatsTracker::new();
        assert!(!tracker.is_in_voice(1, 100).await);

        tracker.voice_join(1, 100).await;
        assert!(tracker.is_in_voice(1, 100).await);

        tracker.voice_leave(1, 100).await;
        assert!(!tracker.is_in_voice(1, 100).await);
    }

    #[tokio::test]
    async fn test_is_in_voice_different_guilds() {
        let tracker = StatsTracker::new();
        tracker.voice_join(1, 100).await;

        assert!(tracker.is_in_voice(1, 100).await);
        assert!(!tracker.is_in_voice(2, 100).await);
    }

    // ── Guild stats ──

    #[tokio::test]
    async fn test_guild_stats_sorted_by_messages() {
        let tracker = StatsTracker::new();
        tracker.record_message(1, 100).await;
        tracker.record_message(1, 200).await;
        tracker.record_message(1, 200).await;
        tracker.record_message(1, 200).await;

        let guild_stats = tracker.get_guild_stats(1).await;
        assert_eq!(guild_stats.len(), 2);
        // uid 200 a 3 messages, uid 100 a 1 -> 200 en premier
        assert_eq!(guild_stats[0].0, 200);
        assert_eq!(guild_stats[0].1.message_count, 3);
        assert_eq!(guild_stats[1].0, 100);
    }

    #[tokio::test]
    async fn test_guild_stats_empty() {
        let tracker = StatsTracker::new();
        let guild_stats = tracker.get_guild_stats(1).await;
        assert!(guild_stats.is_empty());
    }

    #[tokio::test]
    async fn test_guild_stats_only_returns_requested_guild() {
        let tracker = StatsTracker::new();
        tracker.record_message(1, 100).await;
        tracker.record_message(2, 200).await;
        tracker.record_message(3, 300).await;

        let guild_stats = tracker.get_guild_stats(1).await;
        assert_eq!(guild_stats.len(), 1);
        assert_eq!(guild_stats[0].0, 100);
    }

    // ── Counts ──

    #[tokio::test]
    async fn test_tracked_users_count() {
        let tracker = StatsTracker::new();
        assert_eq!(tracker.tracked_users_count(1).await, 0);

        tracker.record_message(1, 100).await;
        tracker.record_message(1, 200).await;
        tracker.record_message(2, 300).await;

        assert_eq!(tracker.tracked_users_count(1).await, 2);
        assert_eq!(tracker.tracked_users_count(2).await, 1);
        assert_eq!(tracker.tracked_users_count(99).await, 0);
    }

    #[tokio::test]
    async fn test_active_voice_sessions() {
        let tracker = StatsTracker::new();
        assert_eq!(tracker.active_voice_sessions(1).await, 0);

        tracker.voice_join(1, 100).await;
        tracker.voice_join(1, 200).await;
        tracker.voice_join(2, 300).await;

        assert_eq!(tracker.active_voice_sessions(1).await, 2);
        assert_eq!(tracker.active_voice_sessions(2).await, 1);

        tracker.voice_leave(1, 100).await;
        assert_eq!(tracker.active_voice_sessions(1).await, 1);
    }

    // ── Concurrent ──

    #[tokio::test]
    async fn test_concurrent_message_recording() {
        let tracker = StatsTracker::new();
        let mut handles = vec![];

        for i in 0..10 {
            let t = tracker.clone();
            handles.push(tokio::spawn(async move {
                for _ in 0..100 {
                    t.record_message(1, i).await;
                }
            }));
        }

        for h in handles {
            h.await.unwrap();
        }

        // 10 users × 100 messages chacun
        let guild_stats = tracker.get_guild_stats(1).await;
        let total: u64 = guild_stats.iter().map(|(_, s)| s.message_count).sum();
        assert_eq!(total, 1000);
        assert_eq!(guild_stats.len(), 10);
    }
}
