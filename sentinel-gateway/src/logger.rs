use std::sync::Arc;

use reqwest::Client;

pub struct GatewayLogger {
    client: Client,
    api_url: String,
    api_key: String,
}

impl GatewayLogger {
    pub fn new(api_url: String) -> Arc<Self> {
        Arc::new(Self {
            client: Client::new(),
            api_url,
            api_key: std::env::var("SENTINEL_API_KEY").unwrap_or_default(),
        })
    }

    pub fn log(&self, level: &str, message: &str, details: serde_json::Value) {
        let url = format!("{}/api/logs", self.api_url);
        let body = serde_json::json!({
            "level": level,
            "bot": "sentinel-gateway",
            "server": "",
            "message": message,
            "category": "websocket",
            "details": details,
        });
        let mut req = self.client.post(url).json(&body);
        if !self.api_key.is_empty() {
            req = req.bearer_auth(&self.api_key);
        }
        tokio::spawn(async move {
            if let Err(e) = req.send().await {
                tracing::debug!(error = %e, "Failed to send gateway log to API");
            }
        });
    }

    pub fn info(&self, message: &str, details: serde_json::Value) {
        self.log("info", message, details);
    }

    pub fn warn(&self, message: &str, details: serde_json::Value) {
        self.log("warn", message, details);
    }

    pub fn error(&self, message: &str, details: serde_json::Value) {
        self.log("error", message, details);
    }
}
