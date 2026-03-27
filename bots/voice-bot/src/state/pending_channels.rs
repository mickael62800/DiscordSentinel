use dashmap::DashMap;
use serenity::model::id::{ChannelId, GuildId, UserId};

#[derive(Clone, Debug)]
pub struct PendingChannel {
    pub owner: UserId,
    pub guild_id: GuildId,
    pub hidden: bool,
}

pub struct PendingChannels {
    map: DashMap<ChannelId, PendingChannel>,
}

impl PendingChannels {
    pub fn new() -> Self {
        Self {
            map: DashMap::new(),
        }
    }

    pub fn remove(&self, text_channel_id: &ChannelId) -> Option<PendingChannel> {
        self.map.remove(text_channel_id).map(|(_, v)| v)
    }

    pub fn toggle_hidden(&self, text_channel_id: &ChannelId) -> Option<bool> {
        let mut entry = self.map.get_mut(text_channel_id)?;
        let pc = entry.value_mut();
        pc.hidden = !pc.hidden;
        Some(pc.hidden)
    }

    pub fn is_owner(&self, text_channel_id: &ChannelId, user_id: UserId) -> bool {
        self.map
            .get(text_channel_id)
            .map(|e| e.value().owner == user_id)
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cid(id: u64) -> ChannelId { ChannelId::new(id) }
    fn uid(id: u64) -> UserId { UserId::new(id) }
    fn gid(id: u64) -> GuildId { GuildId::new(id) }

    fn insert(pc: &PendingChannels, ch: u64, owner: u64) {
        pc.map.insert(cid(ch), PendingChannel {
            owner: uid(owner),
            guild_id: gid(1),
            hidden: false,
        });
    }

    #[test]
    fn test_is_owner() {
        let pc = PendingChannels::new();
        insert(&pc, 10, 1);
        assert!(pc.is_owner(&cid(10), uid(1)));
        assert!(!pc.is_owner(&cid(10), uid(2)));
    }

    #[test]
    fn test_is_owner_unknown_channel() {
        let pc = PendingChannels::new();
        assert!(!pc.is_owner(&cid(999), uid(1)));
    }

    #[test]
    fn test_toggle_hidden() {
        let pc = PendingChannels::new();
        insert(&pc, 10, 1);
        assert_eq!(pc.toggle_hidden(&cid(10)), Some(true));
        assert_eq!(pc.toggle_hidden(&cid(10)), Some(false));
    }

    #[test]
    fn test_toggle_hidden_unknown() {
        let pc = PendingChannels::new();
        assert_eq!(pc.toggle_hidden(&cid(999)), None);
    }

    #[test]
    fn test_remove() {
        let pc = PendingChannels::new();
        insert(&pc, 10, 1);
        let removed = pc.remove(&cid(10));
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().owner, uid(1));
        assert!(pc.remove(&cid(10)).is_none());
    }
}
