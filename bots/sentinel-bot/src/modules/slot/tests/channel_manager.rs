use super::*;

#[test]
fn register_and_get() {
    let mgr = SlotChannelManager::new();
    let user = UserId::new(1);
    let channel = ChannelId::new(100);
    let guild = GuildId::new(200);

    mgr.register(user, channel, guild);
    assert!(mgr.has_active(user));

    let active = mgr.get(user).unwrap();
    assert_eq!(active.channel_id, channel);
    assert_eq!(active.guild_id, guild);
}

#[test]
fn has_active_false_when_not_registered() {
    let mgr = SlotChannelManager::new();
    assert!(!mgr.has_active(UserId::new(42)));
}

#[test]
fn remove_cleans_up() {
    let mgr = SlotChannelManager::new();
    let user = UserId::new(1);
    mgr.register(user, ChannelId::new(100), GuildId::new(200));

    let removed = mgr.remove(user);
    assert!(removed.is_some());
    assert!(!mgr.has_active(user));
    assert_eq!(mgr.count(), 0);
}

#[test]
fn remove_returns_none_when_absent() {
    let mgr = SlotChannelManager::new();
    assert!(mgr.remove(UserId::new(99)).is_none());
}

#[test]
fn find_by_channel() {
    let mgr = SlotChannelManager::new();
    let user = UserId::new(1);
    let channel = ChannelId::new(100);
    mgr.register(user, channel, GuildId::new(200));

    let found = mgr.find_by_channel(channel);
    assert!(found.is_some());
    assert_eq!(found.unwrap().0, user);
}

#[test]
fn find_by_channel_unknown_returns_none() {
    let mgr = SlotChannelManager::new();
    mgr.register(UserId::new(1), ChannelId::new(100), GuildId::new(200));
    assert!(mgr.find_by_channel(ChannelId::new(999)).is_none());
}

#[test]
fn touch_updates_timestamp() {
    let mgr = SlotChannelManager::new();
    let user = UserId::new(1);
    mgr.register(user, ChannelId::new(100), GuildId::new(200));

    // Force timestamp dans le passe.
    // Sur Windows fresh boot, Instant::now() peut etre < 3600s -> overflow.
    // On skip le test si on ne peut pas reculer (rarissime mais robuste).
    let past = match std::time::Instant::now()
        .checked_sub(std::time::Duration::from_secs(3600))
    {
        Some(p) => p,
        None => return, // skip silencieux : uptime systeme trop bas
    };
    if let Some(mut entry) = mgr.active.get_mut(&user) {
        entry.last_activity = past;
    }

    mgr.touch(user);
    assert!(
        mgr.afk_channels(1800).is_empty(),
        "Touch doit reset le timer d activite"
    );
}

#[test]
fn afk_channels_empty_when_recent() {
    let mgr = SlotChannelManager::new();
    mgr.register(UserId::new(1), ChannelId::new(100), GuildId::new(200));
    assert!(mgr.afk_channels(1800).is_empty());
}

#[test]
fn afk_channels_returns_old_entries() {
    let mgr = SlotChannelManager::new();
    let user = UserId::new(1);
    mgr.register(user, ChannelId::new(100), GuildId::new(200));

    let past = match std::time::Instant::now()
        .checked_sub(std::time::Duration::from_secs(3600))
    {
        Some(p) => p,
        None => return, // skip si uptime trop bas (Windows fresh boot)
    };
    if let Some(mut entry) = mgr.active.get_mut(&user) {
        entry.last_activity = past;
    }

    let afk = mgr.afk_channels(1800);
    assert_eq!(afk.len(), 1);
    assert_eq!(afk[0].0, user);
}

#[test]
fn count_reflects_registered_users() {
    let mgr = SlotChannelManager::new();
    assert_eq!(mgr.count(), 0);
    mgr.register(UserId::new(1), ChannelId::new(100), GuildId::new(200));
    mgr.register(UserId::new(2), ChannelId::new(101), GuildId::new(200));
    assert_eq!(mgr.count(), 2);
}

#[test]
fn duplicate_user_registration_replaces() {
    let mgr = SlotChannelManager::new();
    let user = UserId::new(1);
    mgr.register(user, ChannelId::new(100), GuildId::new(200));
    mgr.register(user, ChannelId::new(999), GuildId::new(200));
    assert_eq!(mgr.get(user).unwrap().channel_id, ChannelId::new(999));
    assert_eq!(mgr.count(), 1);
}
