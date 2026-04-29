//! Client API specifique au progression-bot.
//!
//! Phase 7A — Migration gRPC pilote :
//! - Les endpoints **levels** (`add_xp`, `get_user_level`, `get_level_leaderboard`,
//!   `get_all_rewards`) et **stats** (`record_messages`, `record_voice`,
//!   `get_user_stats`, `get_guild_overview`, `get_leaderboard`) passent
//!   desormais par gRPC via `SentinelGrpcClient`.
//! - Les endpoints sans equivalent proto (`get_streak`, `update_streak`,
//!   `get_infractions`) restent sur `BaseApiClient` HTTP — ils seront migres
//!   dans une iteration ulterieure.
//!
//! Le surface publique (noms de methodes + types de retour) est inchangee :
//! handler.rs et commands/* n'ont pas a etre touches.
//!
//! ## Comportement en cas de panne API
//!
//! Tous les appels gRPC sont wrappes dans `SentinelGrpcClient::guarded()` :
//! - apres 5 echecs consecutifs (`Unavailable` / `DeadlineExceeded` / `Internal`)
//!   le circuit breaker s'ouvre pendant 10 s ;
//! - les appels suivants renvoient `Err("API indisponible...")` immediatement,
//!   sans charger l'API ni faire trainer les commandes Discord ;
//! - apres le cooldown, un appel test est autorise (half-open) ;
//! - succes -> referme, echec -> nouveau cooldown.
//!
//! Concretement :
//! - `add_xp` echoue silencieusement (l'XP du message courant est perdu, ce
//!   n'est pas critique — les messages suivants reprendront quand l'API revient).
//! - Les commandes slash `/level`, `/stats`, `/top` repondent
//!   « API indisponible, reessayez dans quelques instants ».
//! - `record_messages`/`record_voice` (fire-and-forget) sont aussi
//!   court-circuites — les compteurs in-memory du `StatsTracker` continuent
//!   d'accumuler localement et seront flushes au prochain tick.

use std::sync::Arc;

use serde::Deserialize;

use sentinel_shared::api_client::BaseApiClient;
use sentinel_shared::grpc_client::{GrpcCallError, SentinelGrpcClient};

use sentinel_proto::common::v1 as proto_common;
use sentinel_proto::progression::v1 as proto_prog;
use sentinel_proto::stats::v1 as proto_stats;

// ── Response DTOs (surface publique inchangee) ──

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct Infraction {
    pub id: String,
    pub guild_id: String,
    pub user_id: UserId,
    pub username: String,
    pub action: String,
    pub reason: Option<String>,
    pub score: f64,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct StreakResponse {
    #[serde(default)]
    pub streak_current: u32,
    #[serde(default)]
    pub streak_best: u32,
    #[serde(default)]
    pub streak_last_day: u32,
    #[serde(default)]
    pub streak_last_year: i32,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct UserStatsResponse {
    pub guild_id: String,
    pub user_id: UserId,
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
    pub user_id: UserId,
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

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct AddXpResponse {
    pub user: UserLevelResponse,
    pub leveled_up: bool,
    pub old_level: i32,
    pub reward_role_id: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct RewardEntry {
    pub id: String,
    pub guild_id: String,
    pub level: i32,
    pub role_id: RoleId,
    pub source: String,
}

// ── Client ──

/// Client API du progression-bot. gRPC en priorite, HTTP pour les endpoints
/// non encore portes (streaks, infractions).
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
        let mut client = self.grpc.stats();
        self.grpc
            .guarded(|| async move {
                client.record_messages(req).await.map(|_| ())
            })
            .await
            .map_err(grpc_err_to_string)
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
        let mut client = self.grpc.stats();
        self.grpc
            .guarded(|| async move {
                client.record_voice(req).await.map(|_| ())
            })
            .await
            .map_err(grpc_err_to_string)
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
        let mut client = self.grpc.stats();
        let resp = self
            .grpc
            .guarded(|| async move {
                client.get_user_stats(req).await.map(|r| r.into_inner())
            })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(resp.stats.map(proto_user_stats_to_response))
    }

    pub async fn get_guild_overview(
        &self,
        guild_id: &str,
    ) -> Result<GuildOverviewResponse, String> {
        let req = proto_stats::GetGuildOverviewRequest {
            guild_id: guild_id.to_string(),
        };
        let mut client = self.grpc.stats();
        let overview = self
            .grpc
            .guarded(|| async move {
                client.get_guild_overview(req).await.map(|r| r.into_inner())
            })
            .await
            .map_err(grpc_err_to_string)?;
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
        let mut client = self.grpc.stats();
        let list = self
            .grpc
            .guarded(|| async move {
                client.get_leaderboard(req).await.map(|r| r.into_inner())
            })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(list.users.into_iter().map(proto_user_stats_to_response).collect())
    }

    // ── Levels / XP (gRPC) ──

    pub async fn add_xp(
        &self,
        guild_id: &str,
        user_id: &str,
        username: &str,
        amount: i64,
        source: &str,
    ) -> Result<AddXpResponse, String> {
        let req = proto_prog::AddXpRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
            username: username.to_string(),
            amount,
            source: xp_source_str_to_proto(source),
        };
        let mut client = self.grpc.progression();
        let resp = self
            .grpc
            .guarded(|| async move {
                client.add_xp(req).await.map(|r| r.into_inner())
            })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(proto_add_xp_to_response(resp))
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
        let mut client = self.grpc.progression();
        let result = self
            .grpc
            .guarded(|| async move {
                client.get_user_level(req).await.map(|r| r.into_inner())
            })
            .await;
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
        let mut client = self.grpc.progression();
        let board = self
            .grpc
            .guarded(|| async move {
                client.get_leaderboard(req).await.map(|r| r.into_inner())
            })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(board
            .users
            .into_iter()
            .map(proto_user_level_to_response)
            .collect())
    }

    pub async fn get_all_rewards(
        &self,
        guild_id: &str,
    ) -> Result<Vec<RewardEntry>, String> {
        let req = proto_prog::GetRewardsRequest {
            guild_id: guild_id.to_string(),
        };
        let mut client = self.grpc.progression();
        let list = self
            .grpc
            .guarded(|| async move {
                client.get_rewards(req).await.map(|r| r.into_inner())
            })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(list
            .rewards
            .into_iter()
            .map(|r| RewardEntry {
                id: r.id,
                guild_id: r.guild_id,
                level: r.level,
                role_id: r.role_id,
                source: proto_xp_source_to_string(r.source),
            })
            .collect())
    }

    // ── HTTP legacy (pas encore migre en proto) ──

    /// Charge les donnees de streak d'un utilisateur. Reste sur HTTP : pas
    /// d'equivalent gRPC dans v1 du proto progression.
    pub async fn get_streak(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<StreakResponse, String> {
        self.base
            .get_json(&format!("/api/levels/{guild_id}/{user_id}/streak"))
            .await
    }

    /// Persiste les donnees de streak. Reste sur HTTP (cf. get_streak).
    pub async fn update_streak(
        &self,
        guild_id: &str,
        user_id: &str,
        current: u32,
        best: u32,
        last_day: u32,
        last_year: i32,
    ) {
        self.base
            .patch_fire_and_forget(
                &format!("/api/levels/{guild_id}/{user_id}/streak"),
                &serde_json::json!({
                    "streak_current": current,
                    "streak_best": best,
                    "streak_last_day": last_day,
                    "streak_last_year": last_year,
                }),
            )
            .await;
    }

    /// Recupere les infractions d'un serveur. Reste sur HTTP (domaine
    /// moderation, pas migre dans cette iteration).
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

fn proto_xp_source_to_string(value: i32) -> String {
    match proto_common::XpSource::try_from(value).unwrap_or(proto_common::XpSource::Unspecified) {
        proto_common::XpSource::Voice => "voice".to_string(),
        _ => "text".to_string(),
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
        // Streaks ne sont pas dans le proto v1 — restent None ici, le bot
        // les recupere via `get_streak` HTTP en complement.
        streak_current: None,
        streak_best: None,
    }
}

fn proto_add_xp_to_response(r: proto_prog::AddXpResponse) -> AddXpResponse {
    AddXpResponse {
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
        old_level: r.old_level,
        reward_role_id: r.reward_role_id,
        source: Some(proto_xp_source_to_string(r.source)),
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

use sentinel_shared::grpc_client::grpc_err_to_string;
use sentinel_api::domain::entities::system::discord_ids::RoleId;
use sentinel_api::domain::entities::system::discord_ids::UserId;

