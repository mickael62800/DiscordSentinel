//! Methodes `ApiClient` pour les vendettas (cf. COUPE_AMELIORATIONS 5.3).

use serde::{Deserialize, Serialize};

use super::ApiClient;
use sentinel_api::domain::entities::system::discord_ids::GuildId;

#[derive(Debug, Serialize)]
pub struct DeclareVendettaBody<'a> {
    pub challenger_id: &'a str,
    pub target_id: &'a str,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DeclaredVendettaResp {
    pub id: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ActiveVendettaResp {
    pub id: String,
    pub guild_id: GuildId,
    pub challenger_id: String,
    pub target_id: String,
    pub declared_at: String,
    pub expires_at: String,
    pub status: String,
    pub resolved_at: Option<String>,
}

impl ApiClient {
    pub async fn declare_vendetta(
        &self,
        guild_id: &str,
        challenger_id: &str,
        target_id: &str,
    ) -> Result<DeclaredVendettaResp, String> {
        let body = DeclareVendettaBody {
            challenger_id,
            target_id,
        };
        self.base
            .post_json(&format!("/api/coude/{guild_id}/vendettas"), &body)
            .await
    }

    pub async fn list_vendettas_by_challenger(
        &self,
        guild_id: &str,
        challenger_id: &str,
    ) -> Result<Vec<ActiveVendettaResp>, String> {
        self.base
            .get_json(&format!(
                "/api/coude/{guild_id}/vendettas/by-challenger/{challenger_id}"
            ))
            .await
    }
}
