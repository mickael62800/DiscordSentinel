/// Intervalle par defaut pour la verification des combats expires (secondes).
const DEFAULT_COMBAT_EXPIRY_CHECK_SECS: u64 = 86400; // 24h
/// Tick du worker cashbox : 1h par defaut. L'API filtre elle-meme les guilds
/// dues (>= 7 jours depuis la derniere redistribution), donc ticker souvent
/// ne cause aucune sur-redistribution.
const DEFAULT_CASHBOX_TICK_SECS: u64 = 3600;
/// Fenetre minimum entre deux redistributions d'une meme guild.
const DEFAULT_CASHBOX_MIN_DAYS: u64 = 7;

#[derive(Clone)]
pub struct WorkerConfig {
    pub database_url: String,
    pub api_url: String,
    pub combat_expiry_check_secs: u64,
    pub discord_bot_token: String,
    pub betting_check_secs: u64,
    pub hp_regen_tick_secs: u64,
    pub cashbox_tick_secs: u64,
    pub cashbox_min_days: u64,
}

impl WorkerConfig {
    pub fn from_env() -> Self {
        use sentinel_worker_common::{load_database_url, load_api_url, load_env};

        Self {
            database_url: load_database_url(),
            api_url: load_api_url(),
            combat_expiry_check_secs: load_env("COMBAT_EXPIRY_CHECK_SECS", DEFAULT_COMBAT_EXPIRY_CHECK_SECS),
            discord_bot_token: std::env::var("COUDE_DISCORD_TOKEN").unwrap_or_default(),
            betting_check_secs: load_env("BETTING_CHECK_SECS", 30),
            hp_regen_tick_secs: load_env("HP_REGEN_TICK_SECS", 300),
            cashbox_tick_secs: load_env("CASHBOX_TICK_SECS", DEFAULT_CASHBOX_TICK_SECS),
            cashbox_min_days: load_env("CASHBOX_MIN_DAYS", DEFAULT_CASHBOX_MIN_DAYS),
        }
    }

    pub fn apply_db_config(&mut self, db: &std::collections::HashMap<String, String>) {
        use sentinel_worker_common::config_or_env;
        self.combat_expiry_check_secs = config_or_env(db, "combat_expiry_check_secs", "COMBAT_EXPIRY_CHECK_SECS", 86400);
        self.betting_check_secs = config_or_env(db, "betting_check_secs", "BETTING_CHECK_SECS", 30);
        self.hp_regen_tick_secs = config_or_env(db, "hp_regen_tick_secs", "HP_REGEN_TICK_SECS", 300);
        self.cashbox_tick_secs = config_or_env(db, "cashbox_tick_secs", "CASHBOX_TICK_SECS", DEFAULT_CASHBOX_TICK_SECS);
        self.cashbox_min_days = config_or_env(db, "cashbox_min_days", "CASHBOX_MIN_DAYS", DEFAULT_CASHBOX_MIN_DAYS);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_check_interval() {
        assert_eq!(DEFAULT_COMBAT_EXPIRY_CHECK_SECS, 86400);
    }
}
