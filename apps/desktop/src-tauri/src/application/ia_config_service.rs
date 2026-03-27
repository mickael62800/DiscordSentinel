use reqwest::Client;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IaConfig {
    pub guild_id: String,
    pub text_enabled: bool,
    pub text_threshold: f64,
    pub vision_enabled: bool,
    pub vision_threshold: f64,
}

pub struct IaConfigService {
    client: Client,
    base_url: String,
    api_key: String,
}

impl IaConfigService {
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

    pub async fn get_config(&self, guild_id: String) -> Result<IaConfig, String> {
        let req = self.auth(
            self.client.get(format!("{}/api/ia-config/{}", self.base_url, guild_id)),
        );
        let resp = req.send().await.map_err(|e| format!("Connection failed: {e}"))?;
        if !resp.status().is_success() {
            return Err(format!("API error: {}", resp.status()));
        }
        resp.json::<IaConfig>().await.map_err(|e| format!("Parse error: {e}"))
    }

    pub async fn save_config(
        &self,
        guild_id: String,
        text_enabled: bool,
        text_threshold: f64,
        vision_enabled: bool,
        vision_threshold: f64,
    ) -> Result<IaConfig, String> {
        let body = serde_json::json!({
            "text_enabled": text_enabled,
            "text_threshold": text_threshold,
            "vision_enabled": vision_enabled,
            "vision_threshold": vision_threshold,
        });

        let req = self.auth(
            self.client
                .put(format!("{}/api/ia-config/{}", self.base_url, guild_id))
                .json(&body),
        );
        let resp = req.send().await.map_err(|e| format!("Connection failed: {e}"))?;
        if !resp.status().is_success() {
            return Err(format!("API error: {}", resp.status()));
        }
        resp.json::<IaConfig>().await.map_err(|e| format!("Parse error: {e}"))
    }
}
