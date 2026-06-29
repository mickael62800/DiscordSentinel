//! Methodes `ApiClient` des leaderboards (`/leaderboard`).
//!
//! Helper prive `leaderboard` + 5 wrappers publics par categorie
//! (richest / thieves / cowards / chaos / level). Le bot appelle
//! uniquement les wrappers, jamais `leaderboard` directement.

use sentinel_proto::coude::v1 as proto_coude;

use super::{grpc_err_to_string, ApiClient, LeaderboardEntry};

impl ApiClient {
    async fn leaderboard(
        &self,
        guild_id: &str,
        category: proto_coude::LeaderboardCategory,
        limit: i64,
    ) -> Result<Vec<LeaderboardEntry>, String> {
        let req = proto_coude::LeaderboardRequest {
            guild_id: guild_id.to_string(),
            category: category as i32,
            limit,
        };
        let list = crate::grpc_call!(self.grpc, coude_social, leaderboard, req)?;
        Ok(list
            .entries
            .into_iter()
            .map(|e| LeaderboardEntry {
                user_id: e.user_id,
                username: e.username,
                value: e.value,
            })
            .collect())
    }

    pub async fn leaderboard_richest(
        &self,
        guild_id: &str,
        limit: i64,
    ) -> Result<Vec<LeaderboardEntry>, String> {
        self.leaderboard(guild_id, proto_coude::LeaderboardCategory::Richest, limit)
            .await
    }

    pub async fn leaderboard_thieves(
        &self,
        guild_id: &str,
        limit: i64,
    ) -> Result<Vec<LeaderboardEntry>, String> {
        self.leaderboard(guild_id, proto_coude::LeaderboardCategory::Thieves, limit)
            .await
    }

    pub async fn leaderboard_cowards(
        &self,
        guild_id: &str,
        limit: i64,
    ) -> Result<Vec<LeaderboardEntry>, String> {
        self.leaderboard(guild_id, proto_coude::LeaderboardCategory::Cowards, limit)
            .await
    }

    pub async fn leaderboard_chaos(
        &self,
        guild_id: &str,
        limit: i64,
    ) -> Result<Vec<LeaderboardEntry>, String> {
        self.leaderboard(guild_id, proto_coude::LeaderboardCategory::Chaos, limit)
            .await
    }

    pub async fn leaderboard_level(
        &self,
        guild_id: &str,
        limit: i64,
    ) -> Result<Vec<LeaderboardEntry>, String> {
        self.leaderboard(guild_id, proto_coude::LeaderboardCategory::Level, limit)
            .await
    }
}
