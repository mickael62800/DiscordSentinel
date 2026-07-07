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

/// Resultat d'un debit de prank (cout lu server-side).
#[derive(Debug, Clone)]
pub enum PrankDebitOutcome {
    Debited { cost: i64, new_balance: i64 },
    InsufficientFunds { cost: i64, balance: i64 },
}

/// Resultat de la penalite d'annulation de combat (calcul + debit server-side).
#[derive(Debug, Clone)]
pub struct CancelPenaltyOutcome {
    pub penalty: i64,
    pub penalty_percent: i32,
    pub new_balance: i64,
}

impl ApiClient {
    /// Debit d'un prank : cout lu server-side (config guild), debit atomique.
    pub async fn prank_debit(
        &self,
        guild_id: &str,
        user_id: &str,
        prank_type: &str,
    ) -> Result<PrankDebitOutcome, String> {
        let req = proto_coude::PrankDebitRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
            prank_type: prank_type.to_string(),
        };
        let r = crate::grpc_call!(self.grpc, coude_economy, prank_debit, req)?;
        Ok(if r.success {
            PrankDebitOutcome::Debited {
                cost: r.cost,
                new_balance: r.balance,
            }
        } else {
            PrankDebitOutcome::InsufficientFunds {
                cost: r.cost,
                balance: r.balance,
            }
        })
    }

    /// Annulation de combat : penalite calculee ET debitee server-side.
    pub async fn apply_cancel_penalty(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<CancelPenaltyOutcome, String> {
        let req = proto_coude::UserInGuildRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
        };
        let r = crate::grpc_call!(self.grpc, coude_economy, apply_cancel_penalty, req)?;
        Ok(CancelPenaltyOutcome {
            penalty: r.penalty,
            penalty_percent: r.penalty_percent,
            new_balance: r.new_balance,
        })
    }

    /// Refus de combat : penalite calculee (depuis la mise) ET debitee
    /// server-side de facon atomique. `mise` est le vrai input (le taux
    /// `refusal_penalty` est lu cote API).
    pub async fn apply_refusal_penalty(
        &self,
        guild_id: &str,
        user_id: &str,
        mise: i64,
    ) -> Result<CancelPenaltyOutcome, String> {
        let req = proto_coude::RefusalPenaltyRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
            mise,
        };
        let r = crate::grpc_call!(self.grpc, coude_economy, apply_refusal_penalty, req)?;
        Ok(CancelPenaltyOutcome {
            penalty: r.penalty,
            penalty_percent: r.penalty_percent,
            new_balance: r.new_balance,
        })
    }

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
        crate::grpc_call!(@unit self.grpc, coude_economy, record_casino_win, req)
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
        crate::grpc_call!(@unit self.grpc, coude_economy, record_casino_loss, req)
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
        let r = crate::grpc_call!(self.grpc, coude_economy, record_casino_faillite, req)?;
        Ok(r.cleared_coins)
    }

    pub async fn count_casino_today(&self, guild_id: &str, user_id: &str) -> Result<u64, String> {
        let req = proto_coude::UserInGuildRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
        };
        let r = crate::grpc_call!(self.grpc, coude_economy, count_casino_today, req)?;
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
        let r = crate::grpc_call!(self.grpc, coude_economy, sum_casino_gains_today, req)?;
        Ok(r.value)
    }

    pub async fn count_steal_today(&self, guild_id: &str, user_id: &str) -> Result<u64, String> {
        let req = proto_coude::UserInGuildRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
        };
        let r = crate::grpc_call!(self.grpc, coude_economy, count_steal_today, req)?;
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
        let resp = crate::grpc_call!(self.grpc, coude_economy, transfer, req)?;
        Ok(resp
            .taunt_events
            .into_iter()
            .map(super::taunt_event_from_proto)
            .collect())
    }

    /// Don de coins taxe : la taxe, le taux et le solde minimum sont lus et
    /// calcules cote API (config guild server-side). Retourne
    /// `(received, tax, taunt_events)`.
    pub async fn gift_coins(
        &self,
        guild_id: &str,
        donor_id: &str,
        target_id: &str,
        amount: i64,
    ) -> Result<(i64, i64, Vec<super::TauntEvent>), String> {
        let req = proto_coude::GiftCoinsRequest {
            guild_id: guild_id.to_string(),
            donor_id: donor_id.to_string(),
            target_id: target_id.to_string(),
            amount,
        };
        let resp = crate::grpc_call!(self.grpc, coude_economy, gift_coins, req)?;
        Ok((
            resp.received,
            resp.tax,
            resp.taunt_events
                .into_iter()
                .map(super::taunt_event_from_proto)
                .collect(),
        ))
    }

    /// Vol reussi : debite la victime et credite le voleur (clamp au
    /// solde victime). Depuis la migration wallet unifie, retourne le
    /// montant reellement vole + la liste des TauntEvents (faillite
    /// cote victime, jackpot cote voleur) pour dispatch via
    /// `taunts_dispatch`. Le taunt "victim streak" reste declenche
    /// separement par `track_steal_victim` (il depend du nombre de
    /// vols subis, pas du montant).
    pub async fn record_steal(
        &self,
        guild_id: &str,
        thief_id: &str,
        victim_id: &str,
        amount: i64,
    ) -> Result<(i64, Vec<super::TauntEvent>), String> {
        let req = proto_coude::StealRequest {
            guild_id: guild_id.to_string(),
            thief_id: thief_id.to_string(),
            victim_id: victim_id.to_string(),
            amount,
        };
        let r = crate::grpc_call!(self.grpc, coude_economy, steal, req)?;
        let taunts = r
            .taunt_events
            .into_iter()
            .map(super::taunt_event_from_proto)
            .collect();
        Ok((r.stolen, taunts))
    }

    /// Penalite de vol rate : debite au plus `amount` coins du voleur
    /// (clamp au solde reel). Remplace l'ancien appel
    /// `record_coins_lost` pour le chemin d'echec de `/voler`. Retourne
    /// `(lost, taunt_events)` (eventuelle faillite cote voleur).
    pub async fn record_steal_fail_penalty(
        &self,
        guild_id: &str,
        thief_id: &str,
        amount: i64,
    ) -> Result<(i64, Vec<super::TauntEvent>), String> {
        let req = proto_coude::StealFailPenaltyRequest {
            guild_id: guild_id.to_string(),
            thief_id: thief_id.to_string(),
            amount,
        };
        let r = crate::grpc_call!(self.grpc, coude_economy, steal_fail_penalty, req)?;
        let taunts = r
            .taunt_events
            .into_iter()
            .map(super::taunt_event_from_proto)
            .collect();
        Ok((r.lost, taunts))
    }
}
