//! Methodes `ApiClient` pour les maledictions (cf. COUPE_AMELIORATIONS 5.1).
//!
//! Endpoints HTTP exposes par l API serveur :
//!   - POST /api/coude/{g}/curses          — cast (random ou kind explicite)
//!   - GET  /api/coude/{g}/curses/{tgt}    — recupere la curse active
//!   - POST /api/coude/{g}/curses/{tgt}/lift — la cible leve sa curse

use serde::{Deserialize, Serialize};

use super::ApiClient;
use sentinel_api::domain::entities::system::discord_ids::GuildId;

#[derive(Debug, Serialize)]
pub struct CastCurseBody<'a> {
    pub source_id: &'a str,
    pub source_username: &'a str,
    pub target_id: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<&'a str>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CastedCurseResp {
    pub id: String,
    pub kind: String,
    pub kind_label: String,
    pub kind_emoji: String,
    pub cost_paid: i64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ActiveCurseResp {
    pub id: String,
    pub guild_id: GuildId,
    pub target_id: String,
    pub source_id: String,
    pub kind: String,
    pub kind_label: String,
    pub kind_emoji: String,
    pub created_at: String,
    pub expires_at: String,
    pub lifted_at: Option<String>,
    pub lifted_by: Option<String>,
}

#[derive(Debug, Serialize)]
struct LiftCurseBody<'a> {
    target_username: &'a str,
}

impl ApiClient {
    pub async fn cast_curse(
        &self,
        guild_id: &str,
        source_id: &str,
        source_username: &str,
        target_id: &str,
        kind: Option<&str>,
    ) -> Result<CastedCurseResp, String> {
        let body = CastCurseBody {
            source_id,
            source_username,
            target_id,
            kind,
        };
        self.base
            .post_json(&format!("/api/coude/{guild_id}/curses"), &body)
            .await
    }

    pub async fn get_active_curse(
        &self,
        guild_id: &str,
        target_id: &str,
    ) -> Result<Option<ActiveCurseResp>, String> {
        self.base
            .get_json(&format!("/api/coude/{guild_id}/curses/{target_id}"))
            .await
    }

    pub async fn lift_own_curse(
        &self,
        guild_id: &str,
        target_id: &str,
        target_username: &str,
    ) -> Result<ActiveCurseResp, String> {
        let body = LiftCurseBody { target_username };
        self.base
            .post_json(
                &format!("/api/coude/{guild_id}/curses/{target_id}/lift"),
                &body,
            )
            .await
    }
}
