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

/// Resultat de l'analyse d'un lot de joins recents.
#[derive(Debug)]
#[allow(dead_code)]
pub struct RaidAnalysis {
    pub similar_names: bool,
    pub high_default_avatar_ratio: bool,
    pub clustered_creation: bool,
    /// Score composite 0-100.
    pub score: u32,
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

/// Distance de Levenshtein entre deux chaines.
pub fn levenshtein(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let a_len = a_chars.len();
    let b_len = b_chars.len();

    if a_len == 0 {
        return b_len;
    }
    if b_len == 0 {
        return a_len;
    }

    let mut prev: Vec<usize> = (0..=b_len).collect();
    let mut curr = vec![0usize; b_len + 1];

    for i in 1..=a_len {
        curr[0] = i;
        for j in 1..=b_len {
            let cost = if a_chars[i - 1] == b_chars[j - 1] {
                0
            } else {
                1
            };
            curr[j] = (prev[j] + 1)
                .min(curr[j - 1] + 1)
                .min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }

    prev[b_len]
}

/// Verifie si un groupe de noms contient des paires avec distance <= max_distance.
/// Limite a 50 noms pour eviter O(n^2) excessif.
pub fn has_similar_usernames(names: &[String], max_distance: usize) -> bool {
    if names.len() < 2 {
        return false;
    }

    // Limiter pour eviter DoS sur gros raids
    let capped = if names.len() > 50 { &names[..50] } else { names };
    let lowered: Vec<String> = capped.iter().map(|n| n.to_lowercase()).collect();

    for i in 0..lowered.len() {
        for j in (i + 1)..lowered.len() {
            if levenshtein(&lowered[i], &lowered[j]) <= max_distance {
                return true;
            }
        }
    }
    false
}

/// Verifie si les timestamps de creation sont clusteres (ecart max entre min et max <= max_spread_secs).
pub fn are_creations_clustered(timestamps: &[i64], max_spread_secs: i64) -> bool {
    if timestamps.len() < 2 {
        return false;
    }

    let min = timestamps.iter().min().copied().unwrap_or(0);
    let max = timestamps.iter().max().copied().unwrap_or(0);

    (max - min) <= max_spread_secs
}

/// Analyse complete d'un lot de joins.
/// - `name_distance` : seuil Levenshtein pour noms similaires
/// - `creation_spread_secs` : ecart max de creation pour cluster (defaut: 3600 = 1h)
pub fn analyze_joins(
    joins: &[JoinInfo],
    name_distance: usize,
    creation_spread_secs: i64,
) -> RaidAnalysis {
    if joins.len() < 2 {
        return RaidAnalysis {
            similar_names: false,
            high_default_avatar_ratio: false,
            clustered_creation: false,
            score: 0,
        };
    }

    let names: Vec<String> = joins.iter().map(|j| j.username.clone()).collect();
    let similar_names = has_similar_usernames(&names, name_distance);

    let default_count = joins.iter().filter(|j| !j.has_avatar).count();
    let high_default_avatar_ratio = (default_count as f64 / joins.len() as f64) > 0.5;

    let timestamps: Vec<i64> = joins.iter().map(|j| j.account_created_timestamp).collect();
    let clustered_creation = are_creations_clustered(&timestamps, creation_spread_secs);

    let mut score: u32 = 0;
    if similar_names {
        score += 40;
    }
    if high_default_avatar_ratio {
        score += 30;
    }
    if clustered_creation {
        score += 30;
    }

    RaidAnalysis {
        similar_names,
        high_default_avatar_ratio,
        clustered_creation,
        score,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Levenshtein ──

    #[test]
    fn levenshtein_identical() {
        assert_eq!(levenshtein("hello", "hello"), 0);
    }

    #[test]
    fn levenshtein_empty_a() {
        assert_eq!(levenshtein("", "hello"), 5);
    }

    #[test]
    fn levenshtein_empty_b() {
        assert_eq!(levenshtein("hello", ""), 5);
    }

    #[test]
    fn levenshtein_both_empty() {
        assert_eq!(levenshtein("", ""), 0);
    }

    #[test]
    fn levenshtein_one_char_diff() {
        assert_eq!(levenshtein("cat", "bat"), 1);
    }

    #[test]
    fn levenshtein_insertion() {
        assert_eq!(levenshtein("cat", "cats"), 1);
    }

    #[test]
    fn levenshtein_deletion() {
        assert_eq!(levenshtein("cats", "cat"), 1);
    }

    #[test]
    fn levenshtein_completely_different() {
        assert_eq!(levenshtein("abc", "xyz"), 3);
    }

    #[test]
    fn levenshtein_raid_usernames() {
        // Noms typiques de raid bots
        assert!(levenshtein("raider001", "raider002") <= 1);
        assert!(levenshtein("freenitro1", "freenitro2") <= 1);
    }

    // ── Similar usernames ──

    #[test]
    fn similar_names_detected() {
        let names = vec![
            "raider001".to_string(),
            "raider002".to_string(),
            "raider003".to_string(),
        ];
        assert!(has_similar_usernames(&names, 2));
    }

    #[test]
    fn similar_names_case_insensitive() {
        let names = vec!["Raider".to_string(), "raider".to_string()];
        assert!(has_similar_usernames(&names, 0));
    }

    #[test]
    fn different_names_no_match() {
        let names = vec![
            "alice".to_string(),
            "bob".to_string(),
            "charlie".to_string(),
        ];
        assert!(!has_similar_usernames(&names, 1));
    }

    #[test]
    fn single_name_no_match() {
        let names = vec!["alice".to_string()];
        assert!(!has_similar_usernames(&names, 2));
    }

    #[test]
    fn empty_names_no_match() {
        let names: Vec<String> = vec![];
        assert!(!has_similar_usernames(&names, 2));
    }

    // ── Clustered creation ──

    #[test]
    fn clustered_creation_within_spread() {
        let ts = vec![1000, 1010, 1020, 1030];
        assert!(are_creations_clustered(&ts, 60));
    }

    #[test]
    fn clustered_creation_outside_spread() {
        let ts = vec![1000, 5000, 10000];
        assert!(!are_creations_clustered(&ts, 60));
    }

    #[test]
    fn clustered_creation_single() {
        let ts = vec![1000];
        assert!(!are_creations_clustered(&ts, 60));
    }

    #[test]
    fn clustered_creation_exact_boundary() {
        let ts = vec![1000, 1060];
        assert!(are_creations_clustered(&ts, 60));
    }

    #[test]
    fn clustered_creation_just_over() {
        let ts = vec![1000, 1061];
        assert!(!are_creations_clustered(&ts, 60));
    }

    // ── Analyze joins ──

    #[test]
    fn analyze_empty() {
        let result = analyze_joins(&[], 2, 3600);
        assert_eq!(result.score, 0);
    }

    #[test]
    fn analyze_single_join() {
        let joins = vec![JoinInfo {
            username: "alice".to_string(),
            has_avatar: true,
            account_created_timestamp: 1000,
        }];
        let result = analyze_joins(&joins, 2, 3600);
        assert_eq!(result.score, 0);
    }

    #[test]
    fn analyze_full_raid_pattern() {
        let joins = vec![
            JoinInfo { username: "raider01".to_string(), has_avatar: false, account_created_timestamp: 1000 },
            JoinInfo { username: "raider02".to_string(), has_avatar: false, account_created_timestamp: 1010 },
            JoinInfo { username: "raider03".to_string(), has_avatar: false, account_created_timestamp: 1020 },
        ];
        let result = analyze_joins(&joins, 2, 3600);
        assert!(result.similar_names);
        assert!(result.high_default_avatar_ratio);
        assert!(result.clustered_creation);
        assert_eq!(result.score, 100);
    }

    #[test]
    fn analyze_only_similar_names() {
        let joins = vec![
            JoinInfo { username: "bot_abc".to_string(), has_avatar: true, account_created_timestamp: 1000 },
            JoinInfo { username: "bot_abd".to_string(), has_avatar: true, account_created_timestamp: 500_000 },
        ];
        let result = analyze_joins(&joins, 2, 3600);
        assert!(result.similar_names);
        assert!(!result.high_default_avatar_ratio);
        assert!(!result.clustered_creation);
        assert_eq!(result.score, 40);
    }

    #[test]
    fn analyze_only_default_avatars() {
        let joins = vec![
            JoinInfo { username: "alice".to_string(), has_avatar: false, account_created_timestamp: 1000 },
            JoinInfo { username: "bob".to_string(), has_avatar: false, account_created_timestamp: 500_000 },
        ];
        let result = analyze_joins(&joins, 1, 3600);
        assert!(!result.similar_names);
        assert!(result.high_default_avatar_ratio);
        assert!(!result.clustered_creation);
        assert_eq!(result.score, 30);
    }

    #[test]
    fn analyze_only_clustered_creation() {
        let joins = vec![
            JoinInfo { username: "alice".to_string(), has_avatar: true, account_created_timestamp: 1000 },
            JoinInfo { username: "bob".to_string(), has_avatar: true, account_created_timestamp: 1010 },
        ];
        let result = analyze_joins(&joins, 1, 3600);
        assert!(!result.similar_names);
        assert!(!result.high_default_avatar_ratio);
        assert!(result.clustered_creation);
        assert_eq!(result.score, 30);
    }

    #[test]
    fn analyze_normal_joins() {
        let joins = vec![
            JoinInfo { username: "alice".to_string(), has_avatar: true, account_created_timestamp: 1_000_000 },
            JoinInfo { username: "bob".to_string(), has_avatar: true, account_created_timestamp: 2_000_000 },
            JoinInfo { username: "charlie".to_string(), has_avatar: true, account_created_timestamp: 3_000_000 },
        ];
        let result = analyze_joins(&joins, 2, 3600);
        assert_eq!(result.score, 0);
    }

    // ── RecentJoinsTracker ──

    #[test]
    fn tracker_record_and_recent() {
        let tracker = RecentJoinsTracker::new(60);
        let guild = GuildId::new(1);
        tracker.record(guild, JoinInfo {
            username: "alice".to_string(),
            has_avatar: true,
            account_created_timestamp: 1000,
        });
        tracker.record(guild, JoinInfo {
            username: "bob".to_string(),
            has_avatar: false,
            account_created_timestamp: 2000,
        });
        let recent = tracker.recent(guild);
        assert_eq!(recent.len(), 2);
    }

    #[test]
    fn tracker_different_guilds() {
        let tracker = RecentJoinsTracker::new(60);
        let guild_a = GuildId::new(1);
        let guild_b = GuildId::new(2);
        tracker.record(guild_a, JoinInfo { username: "a".to_string(), has_avatar: true, account_created_timestamp: 0 });
        tracker.record(guild_b, JoinInfo { username: "b".to_string(), has_avatar: true, account_created_timestamp: 0 });
        assert_eq!(tracker.recent(guild_a).len(), 1);
        assert_eq!(tracker.recent(guild_b).len(), 1);
    }

    #[test]
    fn tracker_reset() {
        let tracker = RecentJoinsTracker::new(60);
        let guild = GuildId::new(1);
        tracker.record(guild, JoinInfo { username: "a".to_string(), has_avatar: true, account_created_timestamp: 0 });
        tracker.reset(guild);
        assert_eq!(tracker.recent(guild).len(), 0);
    }
}
