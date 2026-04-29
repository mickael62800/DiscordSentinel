//! Methodes `ApiClient` des railleries automatiques.
//!
//! Tracking des streaks cote API (win/loss/steal_victim) + config
//! par guild (salon des railleries, opt-outs individuels). Le bot
//! ne fait que transmettre ; la logique "seuil franchi → TauntEvent"
//! vit dans le domain cote API.

use serde::Deserialize;

use sentinel_proto::coude::v1 as proto_coude;

use super::{grpc_err_to_string, taunt_event_from_proto, ApiClient, TauntEvent};
use crate::domain::entities::system::discord_ids::ChannelId;

// ── Migration 139 : DTOs HTTP pour les hooks blackjack + eco ──

#[derive(Debug, Clone, Deserialize)]
struct TauntEventHttpDto {
    channel_id: ChannelId,
    target_user_id: String,
    message: String,
    nickname_suffix: String,
    streak_kind: String,
    streak_value: i32,
}

#[derive(Debug, Deserialize)]
struct MaybeTauntEventHttpDto {
    event: Option<TauntEventHttpDto>,
}

fn dto_to_taunt(dto: TauntEventHttpDto) -> TauntEvent {
    TauntEvent {
        channel_id: dto.channel_id,
        target_user_id: dto.target_user_id,
        message: dto.message,
        nickname_suffix: dto.nickname_suffix,
        streak_kind: dto.streak_kind,
        streak_value: dto.streak_value,
    }
}

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

    // ── Migration 139 : hooks blackjack + eco via HTTP ──

    /// Blackjack naturel (21 en 2 cartes). One-shot.
    pub async fn track_bj_natural(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<TauntEvent>, String> {
        let path = format!("/api/coude/{guild_id}/taunts/bj/natural/{user_id}");
        let resp: MaybeTauntEventHttpDto = self.base.post_json(&path, &serde_json::json!({})).await?;
        Ok(resp.event.map(dto_to_taunt))
    }

    /// Main blackjack gagnee (palier 3/5/10).
    pub async fn track_bj_hand_won(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<TauntEvent>, String> {
        let path = format!("/api/coude/{guild_id}/taunts/bj/won/{user_id}");
        let resp: MaybeTauntEventHttpDto = self.base.post_json(&path, &serde_json::json!({})).await?;
        Ok(resp.event.map(dto_to_taunt))
    }

    /// Bust blackjack (palier 3/5/10).
    pub async fn track_bj_hand_bust(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<TauntEvent>, String> {
        let path = format!("/api/coude/{guild_id}/taunts/bj/bust/{user_id}");
        let resp: MaybeTauntEventHttpDto = self.base.post_json(&path, &serde_json::json!({})).await?;
        Ok(resp.event.map(dto_to_taunt))
    }

    /// Faillite (wallet passe a 0 apres une op). One-shot.
    pub async fn track_bankruptcy(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<TauntEvent>, String> {
        let path = format!("/api/coude/{guild_id}/taunts/eco/bankruptcy/{user_id}");
        let resp: MaybeTauntEventHttpDto = self.base.post_json(&path, &serde_json::json!({})).await?;
        Ok(resp.event.map(dto_to_taunt))
    }

    /// Jackpot (gain > seuil). One-shot si threshold franchi.
    pub async fn track_jackpot(
        &self,
        guild_id: &str,
        user_id: &str,
        amount: i64,
    ) -> Result<Option<TauntEvent>, String> {
        let path = format!("/api/coude/{guild_id}/taunts/eco/jackpot/{user_id}");
        let resp: MaybeTauntEventHttpDto = self
            .base
            .post_json(&path, &serde_json::json!({ "amount": amount }))
            .await?;
        Ok(resp.event.map(dto_to_taunt))
    }

    /// Don genereux (> seuil). One-shot si threshold franchi.
    pub async fn track_generous_donor(
        &self,
        guild_id: &str,
        user_id: &str,
        amount: i64,
    ) -> Result<Option<TauntEvent>, String> {
        let path = format!("/api/coude/{guild_id}/taunts/eco/donor/{user_id}");
        let resp: MaybeTauntEventHttpDto = self
            .base
            .post_json(&path, &serde_json::json!({ "amount": amount }))
            .await?;
        Ok(resp.event.map(dto_to_taunt))
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
