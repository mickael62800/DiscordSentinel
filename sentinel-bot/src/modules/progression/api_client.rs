//! Client API specifique au progression-bot.
//!
//! - Les endpoints **levels** (`record_text_activity`, `record_voice_activity`,
//!   `get_user_level`, `get_level_leaderboard`) et **stats** (`record_messages`,
//!   `record_voice`, `get_user_stats`, `get_guild_overview`, `get_leaderboard`)
//!   passent par gRPC via `SentinelGrpcClient`. Depuis le refactor P0, le bot
//!   n'envoie que des FAITS BRUTS : c'est l'API qui calcule tout l'XP.
//! - Les endpoints sans equivalent proto (`force_monthly_ranking`,
//!   `get_infractions`) restent sur `BaseApiClient` HTTP.

use std::sync::Arc;

use serde::Deserialize;

use crate::shared::api_client::BaseApiClient;
use crate::shared::grpc_client::{GrpcCallError, SentinelGrpcClient};

use sentinel_proto::common::v1 as proto_common;
use sentinel_proto::progression::v1 as proto_prog;
use sentinel_proto::stats::v1 as proto_stats;

// ── Response DTOs (surface publique inchangee) ──

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct Infraction {
    pub id: String,
    pub guild_id: String,
    pub user_id: String,
    pub username: String,
    pub action: String,
    pub reason: Option<String>,
    pub score: f64,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct UserStatsResponse {
    pub guild_id: String,
    pub user_id: String,
    pub username: String,
    pub message_count: u64,
    pub voice_seconds: u64,
    pub voice_hours: f64,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct GuildOverviewResponse {
    pub guild_id: String,
    pub total_messages: u64,
    pub total_voice_seconds: u64,
    pub total_voice_hours: f64,
    pub active_members: u64,
    pub total_infractions: u64,
    pub total_warns: u64,
    pub total_mutes: u64,
    pub total_bans: u64,
    pub top_members: Vec<UserStatsResponse>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct UserLevelResponse {
    pub user_id: String,
    pub username: String,
    pub xp: i64,
    pub level: i32,
    pub xp_current: i64,
    pub xp_needed: i64,
    #[serde(default)]
    pub xp_text: i64,
    #[serde(default)]
    pub level_text: i32,
    #[serde(default)]
    pub xp_text_current: i64,
    #[serde(default)]
    pub xp_text_needed: i64,
    #[serde(default)]
    pub xp_voice: i64,
    #[serde(default)]
    pub level_voice: i32,
    #[serde(default)]
    pub xp_voice_current: i64,
    #[serde(default)]
    pub xp_voice_needed: i64,
    #[serde(default)]
    pub streak_current: Option<i32>,
    #[serde(default)]
    pub streak_best: Option<i32>,
}


/// Reponse a un fait d'activite (texte/vocal) : l'API a calcule tout l'XP.
#[derive(Debug)]
#[allow(dead_code)]
pub struct RecordActivityResponse {
    pub user: UserLevelResponse,
    pub leveled_up: bool,
    pub old_level_global: i32,
    pub xp_gained: i64,
    pub skipped: bool,
    pub streak_current: u32,
}

#[derive(Debug, Deserialize)]
pub struct RankingEntry {
    pub user_id: String,
    pub xp: i64,
}

#[derive(Debug, Deserialize)]
pub struct ForceRankingResponse {
    pub period_label: String,
    #[serde(default)]
    pub note: Option<String>,
    pub text: Vec<RankingEntry>,
    pub voice: Vec<RankingEntry>,
    pub global: Vec<RankingEntry>,
}

// ── Client ──

pub struct ApiClient {
    base: Arc<BaseApiClient>,
    grpc: Arc<SentinelGrpcClient>,
}

impl ApiClient {
    pub fn new(base: Arc<BaseApiClient>, grpc: Arc<SentinelGrpcClient>) -> Self {
        Self { base, grpc }
    }

    // ── Stats (gRPC) ──

    pub async fn record_messages(
        &self,
        guild_id: &str,
        user_id: &str,
        username: &str,
        count: u64,
    ) -> Result<(), String> {
        let req = proto_stats::RecordMessagesRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
            username: username.to_string(),
            count,
        };
        crate::grpc_call!(@unit self.grpc, stats, record_messages, req)
    }

    pub async fn record_voice(
        &self,
        guild_id: &str,
        user_id: &str,
        username: &str,
        seconds: u64,
        channel_id: &str,
        channel_name: &str,
    ) -> Result<(), String> {
        let req = proto_stats::RecordVoiceRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
            username: username.to_string(),
            seconds,
            channel_id: channel_id.to_string(),
            channel_name: channel_name.to_string(),
        };
        crate::grpc_call!(@unit self.grpc, stats, record_voice, req)
    }

    pub async fn get_user_stats(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<UserStatsResponse>, String> {
        let req = proto_stats::GetUserStatsRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
        };
        let resp = crate::grpc_call!(self.grpc, stats, get_user_stats, req)?;
        Ok(resp.stats.map(proto_user_stats_to_response))
    }

    pub async fn get_guild_overview(
        &self,
        guild_id: &str,
    ) -> Result<GuildOverviewResponse, String> {
        let req = proto_stats::GetGuildOverviewRequest {
            guild_id: guild_id.to_string(),
        };
        let overview = crate::grpc_call!(self.grpc, stats, get_guild_overview, req)?;
        Ok(GuildOverviewResponse {
            guild_id: overview.guild_id,
            total_messages: overview.total_messages,
            total_voice_seconds: overview.total_voice_seconds,
            total_voice_hours: overview.total_voice_seconds as f64 / 3600.0,
            active_members: overview.active_members,
            total_infractions: overview.total_infractions,
            total_warns: overview.total_warns,
            total_mutes: overview.total_mutes,
            total_bans: overview.total_bans,
            top_members: overview
                .top_members
                .into_iter()
                .map(proto_user_stats_to_response)
                .collect(),
        })
    }

    pub async fn get_leaderboard(
        &self,
        guild_id: &str,
        limit: u32,
    ) -> Result<Vec<UserStatsResponse>, String> {
        let req = proto_stats::GetLeaderboardRequest {
            guild_id: guild_id.to_string(),
            limit,
        };
        let list = crate::grpc_call!(self.grpc, stats, get_leaderboard, req)?;
        Ok(list
            .users
            .into_iter()
            .map(proto_user_stats_to_response)
            .collect())
    }

    // ── Levels / XP (gRPC) ──

    /// Envoie un FAIT BRUT texte : "un message qualifiant a eu lieu". L'API
    /// calcule le montant d'XP (multiplicateurs channel/role, streak, cooldown).
    pub async fn record_text_activity(
        &self,
        guild_id: &str,
        user_id: &str,
        username: &str,
        channel_id: u64,
        role_ids: &[u64],
    ) -> Result<RecordActivityResponse, String> {
        let req = proto_prog::RecordTextActivityRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
            username: username.to_string(),
            channel_id: channel_id.to_string(),
            role_ids: role_ids.iter().map(|r| r.to_string()).collect(),
        };
        let resp = crate::grpc_call!(self.grpc, progression, record_text_activity, req)?;
        Ok(proto_record_activity_to_response(resp))
    }

    /// Envoie un FAIT BRUT vocal : `seconds` secondes creditables dans le
    /// salon. L'API calcule le montant d'XP (multiplicateurs channel/role).
    pub async fn record_voice_activity(
        &self,
        guild_id: &str,
        user_id: &str,
        username: &str,
        channel_id: u64,
        role_ids: &[u64],
        seconds: u64,
    ) -> Result<RecordActivityResponse, String> {
        let req = proto_prog::RecordVoiceActivityRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
            username: username.to_string(),
            channel_id: channel_id.to_string(),
            role_ids: role_ids.iter().map(|r| r.to_string()).collect(),
            seconds,
        };
        let resp = crate::grpc_call!(self.grpc, progression, record_voice_activity, req)?;
        Ok(proto_record_activity_to_response(resp))
    }

    pub async fn get_user_level(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<UserLevelResponse>, String> {
        let req = proto_prog::GetUserLevelRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
        };
        let result = crate::grpc_call!(@raw self.grpc, progression, get_user_level, req);
        match result {
            Ok(level) => Ok(Some(proto_user_level_to_response(level))),
            Err(GrpcCallError::Status(s)) if s.code() == tonic::Code::NotFound => Ok(None),
            Err(e) => Err(grpc_err_to_string(e)),
        }
    }

    pub async fn get_level_leaderboard(
        &self,
        guild_id: &str,
        limit: u32,
        source: Option<&str>,
    ) -> Result<Vec<UserLevelResponse>, String> {
        let req = proto_prog::GetLeaderboardRequest {
            guild_id: guild_id.to_string(),
            limit: limit as i64,
            source: source
                .map(xp_source_str_to_proto)
                .unwrap_or(proto_common::XpSource::Unspecified as i32),
        };
        let board = crate::grpc_call!(self.grpc, progression, get_leaderboard, req)?;
        Ok(board
            .users
            .into_iter()
            .map(proto_user_level_to_response)
            .collect())
    }

    // ── HTTP legacy ──

    /// Force le calcul du classement mensuel cote API (bypass des gates).
    /// Renvoie les donnees ; le bot fait le rendu + le post Discord.
    pub async fn force_monthly_ranking(
        &self,
        guild_id: &str,
        mois: &str,
    ) -> Result<ForceRankingResponse, String> {
        self.base
            .post_json(
                "/api/analytics/force-monthly-ranking",
                &serde_json::json!({ "guild_id": guild_id, "mois": mois }),
            )
            .await
    }

    pub async fn get_infractions(&self, guild_id: &str) -> Result<Vec<Infraction>, String> {
        self.base
            .get_json(&format!("/infractions/{guild_id}"))
            .await
    }
}

// ── Helpers de conversion proto -> DTOs locaux ──

fn xp_source_str_to_proto(s: &str) -> i32 {
    match s {
        "voice" => proto_common::XpSource::Voice as i32,
        _ => proto_common::XpSource::Text as i32,
    }
}

fn proto_user_level_to_response(u: proto_prog::UserLevel) -> UserLevelResponse {
    UserLevelResponse {
        user_id: u.user_id,
        username: u.username,
        xp: u.xp,
        level: u.level,
        xp_current: u.xp_current,
        xp_needed: u.xp_needed,
        xp_text: u.xp_text,
        level_text: u.level_text,
        xp_text_current: u.xp_text_current,
        xp_text_needed: u.xp_text_needed,
        xp_voice: u.xp_voice,
        level_voice: u.level_voice,
        xp_voice_current: u.xp_voice_current,
        xp_voice_needed: u.xp_voice_needed,
        streak_current: None,
        streak_best: None,
    }
}

fn proto_record_activity_to_response(
    r: proto_prog::RecordActivityResponse,
) -> RecordActivityResponse {
    RecordActivityResponse {
        user: r
            .user
            .map(proto_user_level_to_response)
            .unwrap_or(UserLevelResponse {
                user_id: String::new(),
                username: String::new(),
                xp: 0,
                level: 0,
                xp_current: 0,
                xp_needed: 0,
                xp_text: 0,
                level_text: 0,
                xp_text_current: 0,
                xp_text_needed: 0,
                xp_voice: 0,
                level_voice: 0,
                xp_voice_current: 0,
                xp_voice_needed: 0,
                streak_current: None,
                streak_best: None,
            }),
        leveled_up: r.leveled_up,
        old_level_global: r.old_level_global,
        xp_gained: r.xp_gained,
        skipped: r.skipped,
        streak_current: r.streak_current,
    }
}

fn proto_user_stats_to_response(u: proto_stats::UserStats) -> UserStatsResponse {
    UserStatsResponse {
        guild_id: u.guild_id,
        user_id: u.user_id,
        username: u.username,
        message_count: u.message_count,
        voice_seconds: u.voice_seconds,
        voice_hours: u.voice_seconds as f64 / 3600.0,
        updated_at: u.updated_at,
    }
}

use crate::shared::grpc_client::grpc_err_to_string;
