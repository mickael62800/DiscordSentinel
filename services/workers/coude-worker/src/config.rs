/// Intervalle par defaut pour la verification des combats expires (secondes).
const DEFAULT_COMBAT_EXPIRY_CHECK_SECS: u64 = 86400; // 24h

#[derive(Clone)]
pub struct WorkerConfig {
    pub database_url: String,
    pub api_url: String,
    pub combat_expiry_check_secs: u64,
    pub discord_bot_token: String,
    pub betting_check_secs: u64,
}

impl WorkerConfig {
    pub fn from_env() -> Self {
        let check_secs: u64 = std::env::var("COMBAT_EXPIRY_CHECK_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_COMBAT_EXPIRY_CHECK_SECS);

        let betting_secs: u64 = std::env::var("BETTING_CHECK_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(30);

        Self {
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| {
                    tracing::error!("DATABASE_URL non defini");
                    std::process::exit(1);
                }),
            api_url: std::env::var("API_URL")
                .unwrap_or_else(|_| "http://localhost:3000".into()),
            combat_expiry_check_secs: check_secs,
            discord_bot_token: std::env::var("COUDE_DISCORD_TOKEN").unwrap_or_default(),
            betting_check_secs: betting_secs,
        }
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
