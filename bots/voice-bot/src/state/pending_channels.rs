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

    #[allow(dead_code)]
    pub fn insert(&self, text_channel_id: ChannelId, pending: PendingChannel) {
        self.map.insert(text_channel_id, pending);
    }

    #[allow(dead_code)]
    pub fn get(&self, text_channel_id: &ChannelId) -> Option<PendingChannel> {
        self.map.get(text_channel_id).map(|e| e.value().clone())
    }

    pub fn remove(&self, text_channel_id: &ChannelId) -> Option<PendingChannel> {
        self.map.remove(text_channel_id).map(|(_, v)| v)
    }

    #[allow(dead_code)]
    pub fn contains(&self, text_channel_id: &ChannelId) -> bool {
        self.map.contains_key(text_channel_id)
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
