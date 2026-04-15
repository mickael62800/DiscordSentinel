//! Methodes `ApiClient` des features Phase 9 : protections vol, boosts
//! voleur, railleries (config + track). Extrait du god-object
//! api_client.rs (refactor 2026-04).

use sentinel_proto::coude::v1 as proto_coude;

use super::{
    grpc_err_to_string, taunt_event_from_proto, ApiClient, StealProtection,
    StealProtectionDuration, StealProtectionTrigger, TauntEvent,
};

impl ApiClient {
    // ══════════════════════════════════════════════════════════════════
    // Phase 9 Part B : abonnements anti-vol
    // ══════════════════════════════════════════════════════════════════

    pub async fn list_active_steal_protections(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Vec<StealProtection>, String> {
        let req = proto_coude::UserInGuildRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
        };
        let mut client = self.grpc.coude_inventory();
        let r = self
            .grpc
            .guarded(|| async move {
                client
                    .list_active_steal_protections(req)
                    .await
                    .map(|r| r.into_inner())
            })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(r.protections
            .into_iter()
            .map(|p| StealProtection {
                item_key: p.item_key,
                expires_at: p.expires_at,
            })
            .collect())
    }

    pub async fn price_steal_protection(
        &self,
        item_key: &str,
        duration: StealProtectionDuration,
    ) -> Result<i64, String> {
        let req = proto_coude::PriceStealProtectionRequest {
            item_key: item_key.to_string(),
            duration: duration.as_proto() as i32,
        };
        let mut client = self.grpc.coude_inventory();
        let r = self
            .grpc
            .guarded(|| async move {
                client.price_steal_protection(req).await.map(|r| r.into_inner())
            })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(r.value)
    }

    /// Achete un abonnement de protection. Retourne (cost, expires_at).
    pub async fn buy_steal_protection(
        &self,
        guild_id: &str,
        user_id: &str,
        item_key: &str,
        duration: StealProtectionDuration,
    ) -> Result<(i64, String), String> {
        let req = proto_coude::BuyStealProtectionRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
            item_key: item_key.to_string(),
            duration: duration.as_proto() as i32,
        };
        let mut client = self.grpc.coude_inventory();
        let r = self
            .grpc
            .guarded(|| async move {
                client.buy_steal_protection(req).await.map(|r| r.into_inner())
            })
            .await
            .map_err(grpc_err_to_string)?;
        Ok((r.cost, r.expires_at))
    }

    /// Interroge le serveur pour savoir si une protection a bloque un
    /// vol. L'API roll elle-meme les dés — le bot n'a aucun secret a
    /// garder (au contraire, le voleur ne voit meme pas le nom de
    /// l'item bloquant avant qu'il s'active).
    pub async fn try_trigger_steal_protection(
        &self,
        guild_id: &str,
        target_id: &str,
    ) -> Result<Option<StealProtectionTrigger>, String> {
        let req = proto_coude::UserInGuildRequest {
            guild_id: guild_id.to_string(),
            user_id: target_id.to_string(),
        };
        let mut client = self.grpc.coude_inventory();
        let r = self
            .grpc
            .guarded(|| async move {
                client
                    .try_trigger_steal_protection(req)
                    .await
                    .map(|r| r.into_inner())
            })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(r.trigger.map(|t| StealProtectionTrigger {
            item_key: t.item_key,
            item_name: t.item_name,
            rolled_value: t.rolled_value,
            block_chance_percent: t.block_chance_percent,
        }))
    }

    // ══════════════════════════════════════════════════════════════════
    // Phase 9 Part C : boost voleur
    // ══════════════════════════════════════════════════════════════════

    pub async fn price_steal_boost(
        &self,
        item_key: &str,
        duration: StealProtectionDuration,
    ) -> Result<i64, String> {
        let req = proto_coude::PriceStealBoostRequest {
            item_key: item_key.to_string(),
            duration: duration.as_proto() as i32,
        };
        let mut client = self.grpc.coude_inventory();
        let r = self
            .grpc
            .guarded(|| async move { client.price_steal_boost(req).await.map(|r| r.into_inner()) })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(r.value)
    }

    pub async fn buy_steal_boost(
        &self,
        guild_id: &str,
        user_id: &str,
        item_key: &str,
        duration: StealProtectionDuration,
    ) -> Result<(i64, String), String> {
        let req = proto_coude::BuyStealBoostRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
            item_key: item_key.to_string(),
            duration: duration.as_proto() as i32,
        };
        let mut client = self.grpc.coude_inventory();
        let r = self
            .grpc
            .guarded(|| async move { client.buy_steal_boost(req).await.map(|r| r.into_inner()) })
            .await
            .map_err(grpc_err_to_string)?;
        Ok((r.cost, r.expires_at))
    }

    /// Retourne la somme des roll bonuses des items de boost actifs du
    /// voleur. 0 si aucun item actif. Appele avant un /voler pour
    /// ajouter au thief_total.
    pub async fn get_steal_boost_total(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<i32, String> {
        let req = proto_coude::UserInGuildRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
        };
        let mut client = self.grpc.coude_inventory();
        let r = self
            .grpc
            .guarded(|| async move {
                client.get_steal_boost_total(req).await.map(|r| r.into_inner())
            })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(r.value as i32)
    }

    // ══════════════════════════════════════════════════════════════════
    // Phase 9 Part D : railleries
    // ══════════════════════════════════════════════════════════════════

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
