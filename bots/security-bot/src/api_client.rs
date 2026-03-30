use std::sync::Arc;

use serde::Serialize;

use sentinel_shared::api_client::BaseApiClient;

#[derive(Debug, Serialize)]
pub struct SecurityEvent {
    pub guild_id: String,
    pub event_type: String,
    pub severity: String,
    pub description: String,
    pub user_ids: Vec<String>,
}

/// Client specifique au security-bot, encapsule le BaseApiClient partage.
pub struct ApiClient {
    pub base: Arc<BaseApiClient>,
}

impl ApiClient {
    pub fn new(base: Arc<BaseApiClient>) -> Self {
        Self { base }
    }

    /// Signale un evenement de securite au backend.
    pub async fn report_event(&self, event: &SecurityEvent) -> Result<(), String> {
        let req = self
            .base
            .client()
            .post(format!("{}/api/security/events", self.base.base_url()))
            .json(event);

        self.base
            .auth(req)
            .send()
            .await
            .map_err(|e| format!("Erreur reseau: {e}"))?;

        Ok(())
    }
}
