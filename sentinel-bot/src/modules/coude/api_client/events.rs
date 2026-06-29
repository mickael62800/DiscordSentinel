//! Methodes `ApiClient` liees aux evenements du jeu : saisons,
//! daily chaos, events serveur actifs (happy hour, etc.).
//!
//! Les donnees sont lues depuis `CoudeSocialService`. L'API centralise
//! tout le cycle de vie des saisons (reset tous les 90 jours) et des
//! events, le bot ne fait que lire/logger.

use sentinel_proto::coude::v1 as proto_coude;

use super::{grpc_err_to_string, ApiClient, CurrentSeason, ServerEvent};

impl ApiClient {
    pub async fn get_current_season(&self, guild_id: &str) -> Result<CurrentSeason, String> {
        let req = proto_coude::CurrentSeasonRequest {
            guild_id: guild_id.to_string(),
        };
        let s = crate::grpc_call!(self.grpc, coude_social, current_season, req)?;
        Ok(CurrentSeason {
            season_number: s.season_number,
            started_at: s.started_at,
            ends_at: s.ends_at,
            days_remaining: s.days_remaining,
        })
    }
    pub async fn log_daily_chaos(
        &self,
        guild_id: &str,
        loser_id: &str,
        loser_name: &str,
        winner_id: &str,
        winner_name: &str,
        amount: i64,
    ) -> Result<(), String> {
        let req = proto_coude::LogDailyChaosRequest {
            guild_id: guild_id.to_string(),
            loser_id: loser_id.to_string(),
            loser_name: loser_name.to_string(),
            winner_id: winner_id.to_string(),
            winner_name: winner_name.to_string(),
            amount,
        };
        crate::grpc_call!(@unit self.grpc, coude_social, log_daily_chaos, req)
    }

    pub async fn get_active_events(&self, guild_id: &str) -> Result<Vec<ServerEvent>, String> {
        let req = proto_coude::ListActiveEventsRequest {
            guild_id: guild_id.to_string(),
        };
        let list = crate::grpc_call!(self.grpc, coude_social, list_active_events, req)?;
        Ok(list
            .events
            .into_iter()
            .map(|e| ServerEvent {
                id: e.id,
                guild_id: e.guild_id,
                event_type: String::new(),
                active: e.active,
                expires_at: e.expires_at,
                created_at: e.created_at,
            })
            .collect())
    }
}
