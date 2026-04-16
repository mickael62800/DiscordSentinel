use dashmap::DashMap;
use serenity::model::id::{GuildId, MessageId};

/// Message cache pour retrouver le contenu des messages supprimes.
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct CachedMessage {
    pub author_id: String,
    pub author_name: String,
    pub content: String,
    pub channel_id: String,
}

/// Cache LRU simplifie pour les messages par guild.
pub struct MessageCache {
    cache: DashMap<(GuildId, MessageId), CachedMessage>,
    max_per_guild: usize,
    /// Compteur de messages par guild pour savoir quand evicter.
    counts: DashMap<GuildId, usize>,
}

impl MessageCache {
    pub fn new(max_per_guild: usize) -> Self {
        Self {
            cache: DashMap::new(),
            max_per_guild,
            counts: DashMap::new(),
        }
    }

    /// Stocke un message dans le cache.
    pub fn store(&self, guild_id: GuildId, message_id: MessageId, cached: CachedMessage) {
        // Eviction si on depasse la limite : trier par MessageId croissant
        // (snowflake Discord = plus petit ID = plus vieux message).
        let current_count = self.counts.get(&guild_id).map(|c| *c).unwrap_or(0);
        if current_count >= self.max_per_guild {
            let evict_count = self.max_per_guild / 10;
            let mut guild_keys: Vec<(GuildId, MessageId)> = self
                .cache
                .iter()
                .filter(|e| e.key().0 == guild_id)
                .map(|e| *e.key())
                .collect();
            guild_keys.sort_by_key(|k| k.1);
            let to_remove = &guild_keys[..evict_count.min(guild_keys.len())];

            for key in to_remove {
                self.cache.remove(key);
            }
            if let Some(mut count) = self.counts.get_mut(&guild_id) {
                *count = count.saturating_sub(to_remove.len());
            }
        }

        self.cache.insert((guild_id, message_id), cached);
        let mut count = self.counts.entry(guild_id).or_insert(0);
        *count += 1;

        // Garde de securite globale : empecher le cache de depasser 2x la limite
        if *count > self.max_per_guild * 2 {
            let excess = *count - self.max_per_guild;
            let mut guild_keys: Vec<(GuildId, MessageId)> = self
                .cache
                .iter()
                .filter(|e| e.key().0 == guild_id)
                .map(|e| *e.key())
                .collect();
            guild_keys.sort_by_key(|k| k.1);
            let to_remove = &guild_keys[..excess.min(guild_keys.len())];
            for key in to_remove {
                self.cache.remove(key);
            }
            *count = count.saturating_sub(to_remove.len());
        }
    }

    /// Recupere un message du cache.
    #[allow(dead_code)]
    pub fn get(&self, guild_id: GuildId, message_id: MessageId) -> Option<CachedMessage> {
        self.cache.get(&(guild_id, message_id)).map(|e| e.clone())
    }

    /// Supprime un message du cache.
    pub fn remove(&self, guild_id: GuildId, message_id: MessageId) -> Option<CachedMessage> {
        let removed = self.cache.remove(&(guild_id, message_id));
        if removed.is_some() {
            if let Some(mut count) = self.counts.get_mut(&guild_id) {
                *count = count.saturating_sub(1);
            }
        }
        removed.map(|(_, v)| v)
    }

    /// Nombre de messages en cache pour un guild.
    #[allow(dead_code)]
    pub fn count(&self, guild_id: GuildId) -> usize {
        self.counts
            .get(&guild_id)
            .map(|c| *c)
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_cached(content: &str) -> CachedMessage {
        CachedMessage {
            author_id: "123".to_string(),
            author_name: "Alice".to_string(),
            content: content.to_string(),
            channel_id: "456".to_string(),
        }
    }

    #[test]
    fn store_and_get() {
        let cache = MessageCache::new(100);
        let guild = GuildId::new(1);
        let msg = MessageId::new(42);

        cache.store(guild, msg, make_cached("hello"));

        let result = cache.get(guild, msg);
        assert!(result.is_some());
        assert_eq!(result.unwrap().content, "hello");
    }

    #[test]
    fn get_missing() {
        let cache = MessageCache::new(100);
        let guild = GuildId::new(1);
        let msg = MessageId::new(42);

        assert!(cache.get(guild, msg).is_none());
    }

    #[test]
    fn remove_returns_value() {
        let cache = MessageCache::new(100);
        let guild = GuildId::new(1);
        let msg = MessageId::new(42);

        cache.store(guild, msg, make_cached("hello"));
        let removed = cache.remove(guild, msg);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().content, "hello");

        // Plus dans le cache
        assert!(cache.get(guild, msg).is_none());
    }

    #[test]
    fn remove_missing_returns_none() {
        let cache = MessageCache::new(100);
        assert!(cache.remove(GuildId::new(1), MessageId::new(1)).is_none());
    }

    #[test]
    fn count_tracks_correctly() {
        let cache = MessageCache::new(100);
        let guild = GuildId::new(1);

        assert_eq!(cache.count(guild), 0);

        cache.store(guild, MessageId::new(1), make_cached("a"));
        cache.store(guild, MessageId::new(2), make_cached("b"));
        assert_eq!(cache.count(guild), 2);

        cache.remove(guild, MessageId::new(1));
        assert_eq!(cache.count(guild), 1);
    }

    #[test]
    fn different_guilds_independent() {
        let cache = MessageCache::new(100);
        let guild_a = GuildId::new(1);
        let guild_b = GuildId::new(2);
        let msg = MessageId::new(42);

        cache.store(guild_a, msg, make_cached("guild A"));
        cache.store(guild_b, msg, make_cached("guild B"));

        assert_eq!(cache.get(guild_a, msg).unwrap().content, "guild A");
        assert_eq!(cache.get(guild_b, msg).unwrap().content, "guild B");
    }

    #[test]
    fn eviction_on_overflow() {
        let cache = MessageCache::new(10);
        let guild = GuildId::new(1);

        // Remplir le cache
        for i in 1..=10 {
            cache.store(guild, MessageId::new(i), make_cached(&format!("msg {}", i)));
        }
        assert_eq!(cache.count(guild), 10);

        // Ajouter un 11e devrait declencher l'eviction
        cache.store(guild, MessageId::new(11), make_cached("msg 11"));

        // Le count doit etre <= max (10% evictes + 1 ajoute = 10 - 1 + 1 = 10)
        assert!(cache.count(guild) <= 10);

        // Le dernier message est present
        assert!(cache.get(guild, MessageId::new(11)).is_some());
    }
}
