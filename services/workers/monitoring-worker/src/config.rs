/// Intervalle de check par defaut (secondes).
const DEFAULT_CHECK_INTERVAL_SECS: u64 = 30;

#[derive(Clone)]
pub struct MonitorConfig {
    pub redis_url: String,
    pub api_url: String,
    pub check_interval_secs: u64,
}

impl MonitorConfig {
    pub fn from_env() -> Self {
        Self {
            redis_url: std::env::var("REDIS_URL")
                .unwrap_or_else(|_| {
                    tracing::error!("REDIS_URL non defini, utilisation de la valeur par defaut");
                    "redis://127.0.0.1:6379".into()
                }),
            api_url: std::env::var("API_URL")
                .unwrap_or_else(|_| "http://localhost:3000".into()),
            check_interval_secs: std::env::var("MONITOR_CHECK_INTERVAL")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(DEFAULT_CHECK_INTERVAL_SECS),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_check_interval() {
        assert_eq!(DEFAULT_CHECK_INTERVAL_SECS, 30);
    }
}
