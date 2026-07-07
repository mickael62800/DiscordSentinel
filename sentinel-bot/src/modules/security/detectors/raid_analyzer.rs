#![allow(dead_code)]
use std::time::{Duration, Instant};

use dashmap::DashMap;
use serenity::model::id::GuildId;

/// Metadata d'un utilisateur qui rejoint le serveur.
#[derive(Clone, Debug)]
pub struct JoinInfo {
    pub username: String,
    pub has_avatar: bool,
    pub account_created_timestamp: i64,
}

/// Tracker des joins recents avec metadata (parallele au RaidDetector existant).
pub struct RecentJoinsTracker {
    joins: DashMap<GuildId, Vec<(Instant, JoinInfo)>>,
    window: Duration,
}

impl RecentJoinsTracker {
    pub fn new(window_secs: u64) -> Self {
        Self {
            joins: DashMap::new(),
            window: Duration::from_secs(window_secs),
        }
    }

    /// Enregistre un join et retourne les infos recentes pour ce guild.
    pub fn record(&self, guild_id: GuildId, info: JoinInfo) {
        let now = Instant::now();
        let mut entry = self.joins.entry(guild_id).or_default();
        let list = entry.value_mut();
        list.retain(|(t, _)| now.duration_since(*t) < self.window);
        list.push((now, info));
    }

    /// Retourne les JoinInfo recentes pour un guild.
    pub fn recent(&self, guild_id: GuildId) -> Vec<JoinInfo> {
        let now = Instant::now();
        self.joins
            .get(&guild_id)
            .map(|entry| {
                entry
                    .value()
                    .iter()
                    .filter(|(t, _)| now.duration_since(*t) < self.window)
                    .map(|(_, info)| info.clone())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Reset apres traitement raid.
    pub fn reset(&self, guild_id: GuildId) {
        self.joins.remove(&guild_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── RecentJoinsTracker ──

    #[test]
    fn tracker_record_and_recent() {
        let tracker = RecentJoinsTracker::new(60);
        let guild = GuildId::new(1);
        tracker.record(
            guild,
            JoinInfo {
                username: "alice".to_string(),
                has_avatar: true,
                account_created_timestamp: 1000,
            },
        );
        tracker.record(
            guild,
            JoinInfo {
                username: "bob".to_string(),
                has_avatar: false,
                account_created_timestamp: 2000,
            },
        );
        let recent = tracker.recent(guild);
        assert_eq!(recent.len(), 2);
    }

    #[test]
    fn tracker_different_guilds() {
        let tracker = RecentJoinsTracker::new(60);
        let guild_a = GuildId::new(1);
        let guild_b = GuildId::new(2);
        tracker.record(
            guild_a,
            JoinInfo {
                username: "a".to_string(),
                has_avatar: true,
                account_created_timestamp: 0,
            },
        );
        tracker.record(
            guild_b,
            JoinInfo {
                username: "b".to_string(),
                has_avatar: true,
                account_created_timestamp: 0,
            },
        );
        assert_eq!(tracker.recent(guild_a).len(), 1);
        assert_eq!(tracker.recent(guild_b).len(), 1);
    }

    #[test]
    fn tracker_reset() {
        let tracker = RecentJoinsTracker::new(60);
        let guild = GuildId::new(1);
        tracker.record(
            guild,
            JoinInfo {
                username: "a".to_string(),
                has_avatar: true,
                account_created_timestamp: 0,
            },
        );
        tracker.reset(guild);
        assert_eq!(tracker.recent(guild).len(), 0);
    }
}
