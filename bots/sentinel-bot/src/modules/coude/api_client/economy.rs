//! Methodes `ApiClient` de l'economie secondaire : casino, transferts
//! entre joueurs, vols.
//!
//! - Casino : log des gains/pertes + compteurs journaliers (utilise
//!   par `/blackjack` et les limites quotidiennes).
//! - Transfer : transfert atomique de coins entre deux joueurs
//!   (utilise par `/donner`).
//! - Steal record : atomique SELECT FOR UPDATE + debit/credit pour
//!   `/voler` (utilise apres resolution du roll).

use sentinel_proto::coude::v1 as proto_coude;

use super::{grpc_err_to_string, ApiClient};

impl ApiClient {
    pub async fn record_casino_win(
        &self,
        guild_id: &str,
        user_id: &str,
        gain: i64,
    ) -> Result<(), String> {
        let req = proto_coude::RecordCasinoWinRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
            gain,
        };
        let mut client = self.grpc.coude_economy();
        self.grpc
            .guarded(|| async move { client.record_casino_win(req).await.map(|_| ()) })
            .await
            .map_err(grpc_err_to_string)
    }

    pub async fn record_casino_loss(
        &self,
        guild_id: &str,
        user_id: &str,
        lost: i64,
    ) -> Result<(), String> {
        let req = proto_coude::RecordCasinoLossRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
            lost,
        };
        let mut client = self.grpc.coude_economy();
        self.grpc
            .guarded(|| async move { client.record_casino_loss(req).await.map(|_| ()) })
            .await
            .map_err(grpc_err_to_string)
    }

    pub async fn record_casino_faillite(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<i64, String> {
        let req = proto_coude::RecordCasinoFailliteRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
        };
        let mut client = self.grpc.coude_economy();
        let r = self
            .grpc
            .guarded(|| async move {
                client.record_casino_faillite(req).await.map(|r| r.into_inner())
            })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(r.cleared_coins)
    }

    pub async fn count_casino_today(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<u64, String> {
        let req = proto_coude::UserInGuildRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
        };
        let mut client = self.grpc.coude_economy();
        let r = self
            .grpc
            .guarded(|| async move {
                client.count_casino_today(req).await.map(|r| r.into_inner())
            })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(r.value.max(0) as u64)
    }

    pub async fn sum_casino_gains_today(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<i64, String> {
        let req = proto_coude::UserInGuildRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
        };
        let mut client = self.grpc.coude_economy();
        let r = self
            .grpc
            .guarded(|| async move {
                client.sum_casino_gains_today(req).await.map(|r| r.into_inner())
            })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(r.value)
    }

    pub async fn count_steal_today(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<u64, String> {
        let req = proto_coude::UserInGuildRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
        };
        let mut client = self.grpc.coude_economy();
        let r = self
            .grpc
            .guarded(|| async move {
                client.count_steal_today(req).await.map(|r| r.into_inner())
            })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(r.value.max(0) as u64)
    }

    /// Don de coins entre deux joueurs.
    ///
    /// Depuis la migration wallet unifie cote API, cet endpoint retourne
    /// desormais la liste des `TauntEvent` declenches (faillite cote
    /// emetteur, jackpot cote recepteur, don genereux) pour dispatch
    /// immediat via `taunts_dispatch`. Plus besoin d'appeler
    /// `track_generous_donor` en sequence cote bot.
    pub async fn transfer_coins(
        &self,
        guild_id: &str,
        from_id: &str,
        to_id: &str,
        amount: i64,
    ) -> Result<Vec<super::TauntEvent>, String> {
        let req = proto_coude::TransferRequest {
            guild_id: guild_id.to_string(),
            from_id: from_id.to_string(),
            to_id: to_id.to_string(),
            amount,
        };
        let mut client = self.grpc.coude_economy();
        let resp = self
            .grpc
            .guarded(|| async move { client.transfer(req).await.map(|r| r.into_inner()) })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(resp
            .taunt_events
            .into_iter()
            .map(super::taunt_event_from_proto)
            .collect())
    }

    pub async fn record_steal(
        &self,
        guild_id: &str,
        thief_id: &str,
        victim_id: &str,
        amount: i64,
    ) -> Result<i64, String> {
        let req = proto_coude::StealRequest {
            guild_id: guild_id.to_string(),
            thief_id: thief_id.to_string(),
            victim_id: victim_id.to_string(),
            amount,
        };
        let mut client = self.grpc.coude_economy();
        let r = self
            .grpc
            .guarded(|| async move { client.steal(req).await.map(|r| r.into_inner()) })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(r.stolen)
    }
}
