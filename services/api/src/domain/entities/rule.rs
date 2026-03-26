use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::value_objects::FlagType;

#[derive(Debug, Clone)]
pub struct Rule {
    pub id: Uuid,
    pub guild_id: String,
    pub flag_type: FlagType,
    pub weight: f64,
    pub threshold_warn: f64,
    pub threshold_delete: f64,
    pub threshold_mute: f64,
    pub threshold_ban: f64,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Rule {
    pub fn new(guild_id: String, flag_type: FlagType) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            guild_id,
            weight: Self::default_weight_for(&flag_type),
            flag_type,
            threshold_warn: 2.0,
            threshold_delete: 4.0,
            threshold_mute: 6.0,
            threshold_ban: 9.0,
            enabled: true,
            created_at: now,
            updated_at: now,
        }
    }

    fn default_weight_for(flag_type: &FlagType) -> f64 {
        match flag_type {
            FlagType::Spam => 3.0,
            FlagType::Insult => 5.0,
            FlagType::Link => 1.0,
        }
    }
}
