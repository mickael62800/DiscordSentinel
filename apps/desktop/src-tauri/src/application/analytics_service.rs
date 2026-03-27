use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FullAnalytics {
    pub heatmap: Vec<serde_json::Value>,
    pub action_distribution: Vec<serde_json::Value>,
    pub top_infractors: Vec<serde_json::Value>,
    pub moderation_trend: Vec<serde_json::Value>,
    pub peak_hours: Vec<serde_json::Value>,
}

pub struct AnalyticsService {
    client: Client,
    base_url: String,
    api_key: String,
}

impl AnalyticsService {
    pub fn new(base_url: String, api_key: String) -> Self {
        Self {
            client: Client::new(),
            base_url,
            api_key,
        }
    }

    fn auth(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        if self.api_key.is_empty() {
            req
        } else {
            req.bearer_auth(&self.api_key)
        }
    }

    pub async fn get_full_analytics(
        &self,
        guild_id: Option<String>,
        days: Option<i32>,
    ) -> Result<FullAnalytics, String> {
        let mut url = format!("{}/api/analytics", self.base_url);
        let mut params = Vec::new();
        if let Some(gid) = guild_id {
            params.push(format!("guild_id={gid}"));
        }
        if let Some(d) = days {
            params.push(format!("days={d}"));
        }
        if !params.is_empty() {
            url = format!("{}?{}", url, params.join("&"));
        }

        let req = self.auth(self.client.get(&url));
        let resp = req.send().await.map_err(|e| format!("Connection failed: {e}"))?;
        if !resp.status().is_success() {
            return Err(format!("API error: {}", resp.status()));
        }
        resp.json::<FullAnalytics>()
            .await
            .map_err(|e| format!("Parse error: {e}"))
    }
}
