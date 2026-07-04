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

// ── Organisations ──

#[derive(Debug, Deserialize)]
pub struct Organization {
    pub name: String,
    pub kind_label: String,
    pub emoji: String,
    pub motto: String,
    pub treasury: i64,
}

#[derive(Debug, Deserialize)]
pub struct OrgInfo {
    pub name: String,
    pub kind_label: String,
    pub emoji: String,
    pub motto: String,
    pub treasury: i64,
    pub reputation: i64,
    pub influence: i64,
    pub member_count: i64,
}

#[derive(Debug, Deserialize)]
pub struct OrgMember {
    pub username: String,
    pub role_label: String,
}

#[derive(Debug, Serialize)]
struct CreateOrgBody<'a> {
    founder_user_id: &'a str,
    founder_username: &'a str,
    kind: &'a str,
    name: &'a str,
    motto: &'a str,
}

#[derive(Debug, Serialize)]
struct OrgNameBody<'a> {
    name: &'a str,
}

#[derive(Debug, Serialize)]
struct JoinOrgBody<'a> {
    name: &'a str,
    user_id: &'a str,
    username: &'a str,
}

pub async fn create_org(
    api: &BaseApiClient,
    guild_id: &str,
    founder_user_id: &str,
    founder_username: &str,
    kind: &str,
    name: &str,
    motto: &str,
) -> Result<Organization, String> {
    api.post_json(
        &format!("/api/influence/{guild_id}/orgs"),
        &CreateOrgBody {
            founder_user_id,
            founder_username,
            kind,
            name,
            motto,
        },
    )
    .await
}

pub async fn org_info(
    api: &BaseApiClient,
    guild_id: &str,
    name: &str,
) -> Result<OrgInfo, String> {
    api.post_json(
        &format!("/api/influence/{guild_id}/orgs/info"),
        &OrgNameBody { name },
    )
    .await
}

pub async fn join_org(
    api: &BaseApiClient,
    guild_id: &str,
    name: &str,
    user_id: &str,
    username: &str,
) -> Result<Organization, String> {
    api.post_json(
        &format!("/api/influence/{guild_id}/orgs/join"),
        &JoinOrgBody {
            name,
            user_id,
            username,
        },
    )
    .await
}

pub async fn org_members(
    api: &BaseApiClient,
    guild_id: &str,
    name: &str,
) -> Result<Vec<OrgMember>, String> {
    api.post_json(
        &format!("/api/influence/{guild_id}/orgs/members"),
        &OrgNameBody { name },
    )
    .await
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

// ── Votes / motions ──

#[derive(Debug, Deserialize)]
pub struct MotionState {
    pub motion_id: String,
    pub org_name: String,
    pub title: String,
    pub status: String,
    pub status_label: String,
    pub pour: i64,
    pub contre: i64,
    pub abstention: i64,
}

#[derive(Debug, Serialize)]
struct CreateMotionBody<'a> {
    org_name: &'a str,
    creator_user_id: &'a str,
    creator_username: &'a str,
    title: &'a str,
}

#[derive(Debug, Serialize)]
struct CastVoteBody<'a> {
    motion_id: &'a str,
    user_id: &'a str,
    username: &'a str,
    choice: &'a str,
}

#[derive(Debug, Serialize)]
struct MotionActorBody<'a> {
    motion_id: &'a str,
    user_id: &'a str,
}

pub async fn create_motion(
    api: &BaseApiClient,
    guild_id: &str,
    org_name: &str,
    creator_user_id: &str,
    creator_username: &str,
    title: &str,
) -> Result<MotionState, String> {
    api.post_json(
        &format!("/api/influence/{guild_id}/motions"),
        &CreateMotionBody {
            org_name,
            creator_user_id,
            creator_username,
            title,
        },
    )
    .await
}

pub async fn cast_vote(
    api: &BaseApiClient,
    guild_id: &str,
    motion_id: &str,
    user_id: &str,
    username: &str,
    choice: &str,
) -> Result<MotionState, String> {
    api.post_json(
        &format!("/api/influence/{guild_id}/motions/vote"),
        &CastVoteBody {
            motion_id,
            user_id,
            username,
            choice,
        },
    )
    .await
}

pub async fn close_motion(
    api: &BaseApiClient,
    guild_id: &str,
    motion_id: &str,
    user_id: &str,
) -> Result<MotionState, String> {
    api.post_json(
        &format!("/api/influence/{guild_id}/motions/close"),
        &MotionActorBody { motion_id, user_id },
    )
    .await
}

// ── Capitaux & conversions (Phase 2) ──

#[derive(Debug, Deserialize)]
pub struct CapitalLine {
    pub label: String,
    pub emoji: String,
    pub value: i64,
}

#[derive(Debug, Deserialize)]
pub struct Movement {
    pub emoji: String,
    pub delta: i64,
    pub reason: String,
}

#[derive(Debug, Deserialize)]
pub struct CapitalOverview {
    pub lines: Vec<CapitalLine>,
    pub movements: Vec<Movement>,
}

#[derive(Debug, Deserialize)]
pub struct ConversionOutcome {
    pub source_label: String,
    pub target_label: String,
    pub spent: i64,
    pub gained: i64,
    pub new_source: i64,
    pub new_target: i64,
}

#[derive(Debug, Serialize)]
struct UserBody<'a> {
    user_id: &'a str,
    username: &'a str,
}

#[derive(Debug, Serialize)]
struct ConvertBody<'a> {
    user_id: &'a str,
    username: &'a str,
    kind: &'a str,
    budget: i64,
}

pub async fn view_capital(
    api: &BaseApiClient,
    guild_id: &str,
    user_id: &str,
    username: &str,
) -> Result<CapitalOverview, String> {
    api.post_json(
        &format!("/api/influence/{guild_id}/capital"),
        &UserBody { user_id, username },
    )
    .await
}

pub async fn convert_capital(
    api: &BaseApiClient,
    guild_id: &str,
    user_id: &str,
    username: &str,
    kind: &str,
    budget: i64,
) -> Result<ConversionOutcome, String> {
    api.post_json(
        &format!("/api/influence/{guild_id}/capital/convert"),
        &ConvertBody {
            user_id,
            username,
            kind,
            budget,
        },
    )
    .await
}

// ── Lois (Phase 3) ──

#[derive(Debug, Deserialize)]
pub struct LawState {
    pub law_id: String,
    pub title: String,
    pub body: String,
    pub status: String,
    pub status_label: String,
    pub pour: i64,
    pub contre: i64,
    pub abstention: i64,
}

#[derive(Debug, Serialize)]
struct ProposeLawBody<'a> {
    author_user_id: &'a str,
    author_username: &'a str,
    title: &'a str,
    body: &'a str,
}

#[derive(Debug, Serialize)]
struct LawVoteBody<'a> {
    law_id: &'a str,
    user_id: &'a str,
    username: &'a str,
    choice: &'a str,
}

#[derive(Debug, Serialize)]
struct SetLawMessageBody<'a> {
    law_id: &'a str,
    channel_id: &'a str,
    message_id: &'a str,
}

#[derive(Debug, Serialize)]
struct LawIdBody<'a> {
    law_id: &'a str,
}

pub async fn propose_law(
    api: &BaseApiClient,
    guild_id: &str,
    author_user_id: &str,
    author_username: &str,
    title: &str,
    body: &str,
) -> Result<LawState, String> {
    api.post_json(
        &format!("/api/influence/{guild_id}/laws"),
        &ProposeLawBody { author_user_id, author_username, title, body },
    )
    .await
}

pub async fn law_vote(
    api: &BaseApiClient,
    guild_id: &str,
    law_id: &str,
    user_id: &str,
    username: &str,
    choice: &str,
) -> Result<LawState, String> {
    api.post_json(
        &format!("/api/influence/{guild_id}/laws/vote"),
        &LawVoteBody { law_id, user_id, username, choice },
    )
    .await
}

pub async fn set_law_message(
    api: &BaseApiClient,
    guild_id: &str,
    law_id: &str,
    channel_id: &str,
    message_id: &str,
) -> Result<serde_json::Value, String> {
    api.post_json(
        &format!("/api/influence/{guild_id}/laws/message"),
        &SetLawMessageBody { law_id, channel_id, message_id },
    )
    .await
}

pub async fn law_state(
    api: &BaseApiClient,
    guild_id: &str,
    law_id: &str,
) -> Result<LawState, String> {
    api.post_json(
        &format!("/api/influence/{guild_id}/laws/state"),
        &LawIdBody { law_id },
    )
    .await
}
