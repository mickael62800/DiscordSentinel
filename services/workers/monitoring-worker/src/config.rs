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
                .expect("REDIS_URL requis"),
            api_url: std::env::var("API_URL")
                .unwrap_or_else(|_| "http://localhost:3000".into()),
            check_interval_secs: std::env::var("MONITOR_CHECK_INTERVAL")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(30),
        }
    }
}
