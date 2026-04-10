//! TypeMap keys partagés avec les sous-handlers et les commandes.

use std::sync::Arc;

use dashmap::DashSet;
use serenity::prelude::*;

use crate::anomaly::AnomalyDetector;
use crate::config::Config;
use crate::message_cache::MessageCache;
use crate::weekly_report::WeeklyTracker;

pub struct MessageCacheKey;
impl TypeMapKey for MessageCacheKey {
    type Value = MessageCache;
}

pub struct AnomalyDetectorKey;
impl TypeMapKey for AnomalyDetectorKey {
    type Value = AnomalyDetector;
}

pub struct WeeklyTrackerKey;
impl TypeMapKey for WeeklyTrackerKey {
    type Value = WeeklyTracker;
}

pub struct ConfigKey;
impl TypeMapKey for ConfigKey {
    type Value = Config;
}

/// Cache des user_ids surveillés (rafraîchi toutes les 60s).
pub struct WatchedUserIdsKey;
impl TypeMapKey for WatchedUserIdsKey {
    type Value = Arc<DashSet<String>>;
}
