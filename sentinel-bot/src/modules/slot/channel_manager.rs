//! Gestion des salons temporaires de slot machine (1 par utilisateur actif).
//!
//! Pattern aligne sur blackjack : un user clique "Ouvrir ma machine" dans le
//! panel global -> on cree (ou retourne) son salon perso. Chaque salon a un
//! topic `[slot:{user_id}]` qui permet d identifier le owner cote `find_*`.
//!
//! L auto-cleanup AFK est V2 (cf. `afk_channels` ci-dessous) — pas branche
//! dans cette version.

use std::time::Instant;

use dashmap::DashMap;
use serenity::model::id::{ChannelId, GuildId, UserId};

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct ActiveSlotChannel {
    pub channel_id: ChannelId,
    /// Garde pour reference / metadata (logs, V2 cleanup) — pas lu par le
    /// code actuel mais utile au debug.
    pub guild_id: GuildId,
    pub last_activity: Instant,
}

pub struct SlotChannelManager {
    /// user_id -> son salon slot actif (1 max par user).
    active: DashMap<UserId, ActiveSlotChannel>,
}

impl Default for SlotChannelManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SlotChannelManager {
    pub fn new() -> Self {
        Self { active: DashMap::new() }
    }

    pub fn register(&self, user_id: UserId, channel_id: ChannelId, guild_id: GuildId) {
        self.active.insert(
            user_id,
            ActiveSlotChannel {
                channel_id,
                guild_id,
                last_activity: Instant::now(),
            },
        );
    }

    /// Update timestamp d activite. Appele a chaque spin/daily pour empecher
    /// l auto-cleanup V2 sur un salon en cours d utilisation.
    pub fn touch(&self, user_id: UserId) {
        if let Some(mut entry) = self.active.get_mut(&user_id) {
            entry.last_activity = Instant::now();
        }
    }

    pub fn get(&self, user_id: UserId) -> Option<ActiveSlotChannel> {
        self.active.get(&user_id).map(|e| e.clone())
    }

    /// Utilise par les tests + futurs handlers diag.
    #[allow(dead_code)]
    pub fn has_active(&self, user_id: UserId) -> bool {
        self.active.contains_key(&user_id)
    }

    pub fn find_by_channel(&self, channel_id: ChannelId) -> Option<(UserId, ActiveSlotChannel)> {
        self.active
            .iter()
            .find(|entry| entry.value().channel_id == channel_id)
            .map(|entry| (*entry.key(), entry.value().clone()))
    }

    pub fn remove(&self, user_id: UserId) -> Option<ActiveSlotChannel> {
        self.active.remove(&user_id).map(|(_, v)| v)
    }

    /// Retourne les salons inactifs depuis plus de `timeout_secs`.
    /// Utilise par le cleanup background task (timeout global).
    #[allow(dead_code)]
    pub fn afk_channels(&self, timeout_secs: u64) -> Vec<(UserId, ActiveSlotChannel)> {
        let timeout = std::time::Duration::from_secs(timeout_secs);
        let now = Instant::now();
        self.active
            .iter()
            .filter(|entry| now.duration_since(entry.value().last_activity) >= timeout)
            .map(|entry| (*entry.key(), entry.value().clone()))
            .collect()
    }

    /// Snapshot de tous les salons actifs. Utilise par le cleanup background
    /// task qui applique un timeout PAR GUILD (lu en config), et a donc besoin
    /// de voir tous les salons + leur inactivite pour decider lesquels fermer.
    pub fn snapshot(&self) -> Vec<(UserId, ActiveSlotChannel)> {
        self.active
            .iter()
            .map(|entry| (*entry.key(), entry.value().clone()))
            .collect()
    }

    /// Secondes d'inactivite d'un salon (depuis le dernier `touch`).
    pub fn idle_secs(channel: &ActiveSlotChannel) -> u64 {
        Instant::now()
            .duration_since(channel.last_activity)
            .as_secs()
    }

    /// Nombre de salons actifs. Utilise par tests + futur monitoring.
    #[allow(dead_code)]
    pub fn count(&self) -> usize {
        self.active.len()
    }
}

#[cfg(test)]
#[path = "tests/channel_manager.rs"]
mod tests;
