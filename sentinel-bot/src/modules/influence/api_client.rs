//! Appels HTTP du module Influence vers sentinel-api.

use serde::{Deserialize, Serialize};

use crate::shared::api_client::BaseApiClient;

#[derive(Debug, Serialize)]
struct ViewProfileBody<'a> {
    viewer_user_id: &'a str,
    target_user_id: &'a str,
    target_username: &'a str,
}

#[derive(Debug, Deserialize)]
pub struct CapitalView {
    pub tier: String,
    pub stars: String,
    pub exact: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct ProfileView {
    pub username: String,
    pub is_self: bool,
    pub influence: CapitalView,
    pub money: CapitalView,
    pub reputation_tier: String,
    pub reputation_exact: Option<i64>,
    pub information: CapitalView,
    pub network: CapitalView,
}

/// POST /api/influence/{guild}/profile
pub async fn view_profile(
    api: &BaseApiClient,
    guild_id: &str,
    viewer_user_id: &str,
    target_user_id: &str,
    target_username: &str,
) -> Result<ProfileView, String> {
    api.post_json(
        &format!("/api/influence/{guild_id}/profile"),
        &ViewProfileBody {
            viewer_user_id,
            target_user_id,
            target_username,
        },
    )
    .await
}
