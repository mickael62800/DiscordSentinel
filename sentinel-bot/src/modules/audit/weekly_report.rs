use dashmap::DashMap;
use serenity::model::id::GuildId;

/// Statistiques hebdomadaires par guild.
#[derive(Debug, Clone, Default)]
pub struct WeeklyStats {
    pub member_joins: u64,
    pub member_leaves: u64,
    pub bans: u64,
    pub messages_deleted: u64,
    pub messages_edited: u64,
    pub role_changes: u64,
    pub channel_changes: u64,
    pub voice_events: u64,
    pub anomalies: u64,
}

/// Champs incrementables.
pub enum StatField {
    MemberJoin,
    MemberLeave,
    Ban,
    MessageDeleted,
    MessageEdited,
    RoleChange,
    ChannelChange,
    VoiceEvent,
    Anomaly,
}

/// Tracker de stats hebdomadaires. Drain chaque semaine pour generer le rapport.
pub struct WeeklyTracker {
    stats: DashMap<GuildId, WeeklyStats>,
}

impl WeeklyTracker {
    pub fn new() -> Self {
        Self {
            stats: DashMap::new(),
        }
    }

    /// Incremente un compteur pour un guild.
    pub fn increment(&self, guild_id: GuildId, field: StatField) {
        let mut entry = self.stats.entry(guild_id).or_default();
        let s = entry.value_mut();
        match field {
            StatField::MemberJoin => s.member_joins += 1,
            StatField::MemberLeave => s.member_leaves += 1,
            StatField::Ban => s.bans += 1,
            StatField::MessageDeleted => s.messages_deleted += 1,
            StatField::MessageEdited => s.messages_edited += 1,
            StatField::RoleChange => s.role_changes += 1,
            StatField::ChannelChange => s.channel_changes += 1,
            StatField::VoiceEvent => s.voice_events += 1,
            StatField::Anomaly => s.anomalies += 1,
        }
    }

    /// Incremente le compteur de messages supprimes de `n`.
    pub fn increment_deleted(&self, guild_id: GuildId, n: u64) {
        let mut entry = self.stats.entry(guild_id).or_default();
        entry.value_mut().messages_deleted += n;
    }

    /// Retourne un snapshot des stats courantes pour un guild sans les vider.
    pub fn snapshot(&self, guild_id: GuildId) -> WeeklyStats {
        self.stats
            .get(&guild_id)
            .map(|entry| entry.value().clone())
            .unwrap_or_default()
    }

    /// Drain toutes les stats et les retourne. Remet les compteurs a zero.
    #[allow(dead_code)]
    pub fn take_all(&self) -> Vec<(GuildId, WeeklyStats)> {
        let keys: Vec<GuildId> = self.stats.iter().map(|e| *e.key()).collect();
        let mut results = Vec::with_capacity(keys.len());
        for key in keys {
            if let Some((gid, stats)) = self.stats.remove(&key) {
                results.push((gid, stats));
            }
        }
        results
    }
}

/// Formate les stats en texte pour un embed Discord.
#[allow(dead_code)]
pub fn format_report(stats: &WeeklyStats) -> String {
    format!(
        "\
Membres: +{} / -{}\n\
Bans: {}\n\
Messages supprimes: {}\n\
Messages edites: {}\n\
Changements de roles: {}\n\
Changements de channels: {}\n\
Evenements vocaux: {}\n\
Anomalies detectees: {}",
        stats.member_joins,
        stats.member_leaves,
        stats.bans,
        stats.messages_deleted,
        stats.messages_edited,
        stats.role_changes,
        stats.channel_changes,
        stats.voice_events,
        stats.anomalies,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn increment_and_take() {
        let tracker = WeeklyTracker::new();
        let guild = GuildId::new(1);

        tracker.increment(guild, StatField::MemberJoin);
        tracker.increment(guild, StatField::MemberJoin);
        tracker.increment(guild, StatField::Ban);
        tracker.increment(guild, StatField::MessageDeleted);

        let results = tracker.take_all();
        assert_eq!(results.len(), 1);

        let (gid, stats) = &results[0];
        assert_eq!(*gid, guild);
        assert_eq!(stats.member_joins, 2);
        assert_eq!(stats.bans, 1);
        assert_eq!(stats.messages_deleted, 1);
    }

    #[test]
    fn take_drains() {
        let tracker = WeeklyTracker::new();
        let guild = GuildId::new(1);

        tracker.increment(guild, StatField::MemberJoin);
        assert_eq!(tracker.take_all().len(), 1);
        // Apres drain, plus de stats
        assert_eq!(tracker.take_all().len(), 0);
    }

    #[test]
    fn increment_deleted_bulk() {
        let tracker = WeeklyTracker::new();
        let guild = GuildId::new(1);

        tracker.increment_deleted(guild, 50);
        tracker.increment_deleted(guild, 30);

        let results = tracker.take_all();
        assert_eq!(results[0].1.messages_deleted, 80);
    }

    #[test]
    fn multiple_guilds() {
        let tracker = WeeklyTracker::new();
        let guild_a = GuildId::new(1);
        let guild_b = GuildId::new(2);

        tracker.increment(guild_a, StatField::Ban);
        tracker.increment(guild_b, StatField::MemberJoin);
        tracker.increment(guild_b, StatField::MemberJoin);

        let results = tracker.take_all();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn format_report_output() {
        let stats = WeeklyStats {
            member_joins: 15,
            member_leaves: 3,
            bans: 2,
            messages_deleted: 42,
            messages_edited: 18,
            role_changes: 5,
            channel_changes: 1,
            voice_events: 100,
            anomalies: 0,
        };
        let report = format_report(&stats);
        assert!(report.contains("Membres: +15 / -3"));
        assert!(report.contains("Bans: 2"));
        assert!(report.contains("Messages supprimes: 42"));
        assert!(report.contains("Anomalies detectees: 0"));
    }

    #[test]
    fn format_report_zeros() {
        let stats = WeeklyStats::default();
        let report = format_report(&stats);
        assert!(report.contains("Membres: +0 / -0"));
    }
}
