//! Methodes `ApiClient` des railleries automatiques.
//!
//! Tracking des streaks cote API (win/loss/steal_victim) + config
//! par guild (salon des railleries, opt-outs individuels). Le bot
//! ne fait que transmettre ; la logique "seuil franchi → TauntEvent"
//! vit dans le domain cote API.

use sentinel_proto::coude::v1 as proto_coude;

use super::{grpc_err_to_string, taunt_event_from_proto, ApiClient, TauntEvent};

impl ApiClient {
    /// Tracke un vol reussi : incremente le steal_victim_streak de la
    /// victime et retourne un TauntEvent si un palier est franchi.
    pub async fn track_steal_victim(
        &self,
        guild_id: &str,
        victim_id: &str,
    ) -> Result<Option<TauntEvent>, String> {
        let req = proto_coude::TrackStealVictimRequest {
            guild_id: guild_id.to_string(),
            victim_id: victim_id.to_string(),
        };
        let mut client = self.grpc.coude_social();
        let r = self
            .grpc
            .guarded(|| async move {
                client.track_steal_victim(req).await.map(|r| r.into_inner())
            })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(r.event.map(taunt_event_from_proto))
    }

    /// Reset le steal_victim_streak (protection a bloque).
    pub async fn track_steal_defended(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<(), String> {
        let req = proto_coude::UserInGuildRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
        };
        let mut client = self.grpc.coude_social();
        self.grpc
            .guarded(|| async move { client.track_steal_defended(req).await.map(|_| ()) })
            .await
            .map_err(grpc_err_to_string)
    }

    pub async fn set_taunts_opt_out(
        &self,
        guild_id: &str,
        user_id: &str,
        opted_out: bool,
    ) -> Result<(), String> {
        let req = proto_coude::SetTauntsOptOutRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
            opted_out,
        };
        let mut client = self.grpc.coude_social();
        self.grpc
            .guarded(|| async move { client.set_taunts_opt_out(req).await.map(|_| ()) })
            .await
            .map_err(grpc_err_to_string)
    }

    pub async fn set_taunts_channel(
        &self,
        guild_id: &str,
        channel_id: Option<&str>,
    ) -> Result<(), String> {
        let req = proto_coude::SetTauntsChannelRequest {
            guild_id: guild_id.to_string(),
            channel_id: channel_id.map(|s| s.to_string()),
        };
        let mut client = self.grpc.coude_social();
        self.grpc
            .guarded(|| async move { client.set_taunts_channel(req).await.map(|_| ()) })
            .await
            .map_err(grpc_err_to_string)
    }
}
