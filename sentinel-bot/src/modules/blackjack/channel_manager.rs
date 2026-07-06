use std::time::Instant;

use dashmap::DashMap;
use serenity::model::id::{ChannelId, GuildId, UserId};

/// Gere les channels de blackjack actifs.
/// Chaque joueur a un channel prive qui est supprime a la fin du jeu ou apres AFK.
pub struct ChannelManager {
    /// user_id -> (channel_id, guild_id, game_id, last_activity)
    active: DashMap<UserId, ActiveTable>,
    /// Verrou in-flight par joueur : empeche le double-traitement concurrent
    /// d'une action (Hit/Stand/Double) sur double-clic / retry Discord.
    in_flight: DashMap<UserId, ()>,
}

/// Garde RAII du verrou d'action d'un joueur. Le verrou est relache
/// automatiquement au Drop (fin du handler, quel que soit le chemin de sortie).
pub struct ActionGuard {
    mgr: std::sync::Arc<ChannelManager>,
    user: UserId,
}

impl ActionGuard {
    /// Tente d'acquerir le verrou d'action. Renvoie `None` si une action est
    /// deja en cours de traitement pour ce joueur (double-clic).
    pub fn try_acquire(mgr: std::sync::Arc<ChannelManager>, user: UserId) -> Option<Self> {
        use dashmap::mapref::entry::Entry;
        // Scope l'emprunt de l'entry AVANT de deplacer `mgr` dans Self.
        let acquired = match mgr.in_flight.entry(user) {
            Entry::Occupied(_) => false,
            Entry::Vacant(v) => {
                v.insert(());
                true
            }
        };
        if acquired {
            Some(Self { mgr, user })
        } else {
            None
        }
    }
}

impl Drop for ActionGuard {
    fn drop(&mut self) {
        self.mgr.in_flight.remove(&self.user);
    }
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct ActiveTable {
    pub channel_id: ChannelId,
    pub guild_id: GuildId,
    pub game_id: Option<String>,
    pub last_activity: Instant,
}

#[allow(dead_code)]
impl ChannelManager {
    pub fn new() -> Self {
        Self {
            active: DashMap::new(),
            in_flight: DashMap::new(),
        }
    }

    /// Enregistre un nouveau channel de blackjack pour un joueur.
    pub fn register(&self, user_id: UserId, channel_id: ChannelId, guild_id: GuildId) {
        self.active.insert(
            user_id,
            ActiveTable {
                channel_id,
                guild_id,
                game_id: None,
                last_activity: Instant::now(),
            },
        );
    }

    /// Associe un game_id au channel du joueur.
    pub fn set_game_id(&self, user_id: UserId, game_id: String) {
        if let Some(mut entry) = self.active.get_mut(&user_id) {
            entry.game_id = Some(game_id);
            entry.last_activity = Instant::now();
        }
    }

    /// Met a jour le timestamp d'activite.
    pub fn touch(&self, user_id: UserId) {
        if let Some(mut entry) = self.active.get_mut(&user_id) {
            entry.last_activity = Instant::now();
        }
    }

    /// Verifie si un joueur a un channel actif.
    pub fn has_active(&self, user_id: UserId) -> bool {
        self.active.contains_key(&user_id)
    }

    /// Recupere le channel actif d'un joueur.
    pub fn get(&self, user_id: UserId) -> Option<ActiveTable> {
        self.active.get(&user_id).map(|e| e.clone())
    }

    /// Trouve un joueur par channel_id.
    pub fn find_by_channel(&self, channel_id: ChannelId) -> Option<(UserId, ActiveTable)> {
        self.active
            .iter()
            .find(|entry| entry.value().channel_id == channel_id)
            .map(|entry| (*entry.key(), entry.value().clone()))
    }

    /// Supprime le channel d'un joueur.
    pub fn remove(&self, user_id: UserId) -> Option<ActiveTable> {
        self.active.remove(&user_id).map(|(_, v)| v)
    }

    /// Retourne les channels AFK (inactifs depuis plus de `timeout_secs`).
    pub fn afk_channels(&self, timeout_secs: u64) -> Vec<(UserId, ActiveTable)> {
        let timeout = std::time::Duration::from_secs(timeout_secs);
        let now = Instant::now();
        self.active
            .iter()
            .filter(|entry| now.duration_since(entry.value().last_activity) >= timeout)
            .map(|entry| (*entry.key(), entry.value().clone()))
            .collect()
    }

    /// Nombre de tables actives.
    pub fn count(&self) -> usize {
        self.active.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_and_get() {
        let mgr = ChannelManager::new();
        let user = UserId::new(1);
        let channel = ChannelId::new(100);
        let guild = GuildId::new(200);

        mgr.register(user, channel, guild);
        assert!(mgr.has_active(user));

        let table = mgr.get(user).unwrap();
        assert_eq!(table.channel_id, channel);
        assert_eq!(table.guild_id, guild);
        assert!(table.game_id.is_none());
    }

    #[test]
    fn set_game_id() {
        let mgr = ChannelManager::new();
        let user = UserId::new(1);
        mgr.register(user, ChannelId::new(100), GuildId::new(200));

        mgr.set_game_id(user, "game-123".into());
        let table = mgr.get(user).unwrap();
        assert_eq!(table.game_id.unwrap(), "game-123");
    }

    #[test]
    fn remove_cleans_up() {
        let mgr = ChannelManager::new();
        let user = UserId::new(1);
        mgr.register(user, ChannelId::new(100), GuildId::new(200));

        let removed = mgr.remove(user);
        assert!(removed.is_some());
        assert!(!mgr.has_active(user));
    }

    #[test]
    fn find_by_channel() {
        let mgr = ChannelManager::new();
        let user = UserId::new(1);
        let channel = ChannelId::new(100);
        mgr.register(user, channel, GuildId::new(200));

        let found = mgr.find_by_channel(channel);
        assert!(found.is_some());
        assert_eq!(found.unwrap().0, user);
    }

    #[test]
    fn find_by_channel_unknown() {
        let mgr = ChannelManager::new();
        assert!(mgr.find_by_channel(ChannelId::new(999)).is_none());
    }

    #[test]
    fn afk_channels_empty_when_recent() {
        let mgr = ChannelManager::new();
        mgr.register(UserId::new(1), ChannelId::new(100), GuildId::new(200));
        assert!(mgr.afk_channels(1800).is_empty());
    }

    #[test]
    fn afk_channels_returns_old() {
        let mgr = ChannelManager::new();
        let user = UserId::new(1);
        mgr.register(user, ChannelId::new(100), GuildId::new(200));

        // Forcer le timestamp dans le passe (skip si uptime trop bas)
        let past = match Instant::now().checked_sub(std::time::Duration::from_secs(3600)) {
            Some(p) => p,
            None => return,
        };
        if let Some(mut entry) = mgr.active.get_mut(&user) {
            entry.last_activity = past;
        }

        let afk = mgr.afk_channels(1800);
        assert_eq!(afk.len(), 1);
        assert_eq!(afk[0].0, user);
    }

    #[test]
    fn touch_resets_timer() {
        let mgr = ChannelManager::new();
        let user = UserId::new(1);
        mgr.register(user, ChannelId::new(100), GuildId::new(200));

        // Forcer vieux (skip si uptime trop bas)
        let past = match Instant::now().checked_sub(std::time::Duration::from_secs(3600)) {
            Some(p) => p,
            None => return,
        };
        if let Some(mut entry) = mgr.active.get_mut(&user) {
            entry.last_activity = past;
        }

        mgr.touch(user);
        assert!(
            mgr.afk_channels(1800).is_empty(),
            "Touch doit remettre a zero le timer"
        );
    }

    #[test]
    fn count() {
        let mgr = ChannelManager::new();
        assert_eq!(mgr.count(), 0);
        mgr.register(UserId::new(1), ChannelId::new(100), GuildId::new(200));
        mgr.register(UserId::new(2), ChannelId::new(101), GuildId::new(200));
        assert_eq!(mgr.count(), 2);
    }

    #[test]
    fn duplicate_user_replaces() {
        let mgr = ChannelManager::new();
        let user = UserId::new(1);
        mgr.register(user, ChannelId::new(100), GuildId::new(200));
        mgr.register(user, ChannelId::new(999), GuildId::new(200));
        assert_eq!(mgr.get(user).unwrap().channel_id, ChannelId::new(999));
        assert_eq!(mgr.count(), 1);
    }
}
