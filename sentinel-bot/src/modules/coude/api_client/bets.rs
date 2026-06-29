//! Methodes `ApiClient` des paris (`/pari`).
//!
//! Pari-mutuel sur les combats : placement, liste des paris d'un
//! combat, resolution (redistribution) et refund. Le bot est thin :
//! toute la logique pari-mutuel (commission 15 %, split 10/5/85 entre
//! gagnant combattant / perdant combattant / parieurs gagnants) vit
//! cote API dans le domain `coude_bet`.

use sentinel_proto::coude::v1 as proto_coude;

use super::{
    grpc_err_to_string, proto_combat_to_dto, taunt_event_from_proto, ApiClient, Bet, BetResult,
    Combat, FighterBetBonus, TauntEvent,
};

impl ApiClient {
    /// Place un pari. Retourne les TauntEvents declenches (faillite parieur
    /// si le debit de mise fait passer son solde a zero).
    pub async fn place_bet(
        &self,
        guild_id: &str,
        combat_id: &str,
        bettor_id: &str,
        bettor_name: &str,
        backed_id: &str,
        amount: i64,
    ) -> Result<Vec<TauntEvent>, String> {
        let req = proto_coude::PlaceBetRequest {
            guild_id: guild_id.to_string(),
            combat_id: combat_id.to_string(),
            bettor_id: bettor_id.to_string(),
            bettor_name: bettor_name.to_string(),
            backed_id: backed_id.to_string(),
            amount,
        };
        let mut client = self.grpc.coude_bets();
        let resp = self
            .grpc
            .guarded(|| async move { client.place(req).await.map(|r| r.into_inner()) })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(resp
            .taunt_events
            .into_iter()
            .map(taunt_event_from_proto)
            .collect())
    }

    pub async fn get_combat_bets(&self, combat_id: &str) -> Result<Vec<Bet>, String> {
        let req = proto_coude::ListForCombatRequest {
            combat_id: combat_id.to_string(),
        };
        let mut client = self.grpc.coude_bets();
        let list = self
            .grpc
            .guarded(|| async move { client.list_for_combat(req).await.map(|r| r.into_inner()) })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(list
            .bets
            .into_iter()
            .map(|b| Bet {
                id: b.id.to_string(),
                combat_id: b.combat_id,
                bettor_id: b.bettor_id,
                bettor_name: b.bettor_name,
                backed_id: b.backed_id,
                amount: b.amount,
            })
            .collect())
    }

    pub async fn get_betting_combat_for_player(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<Combat>, String> {
        let req = proto_coude::GetBettingForParticipantRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
        };
        let mut client = self.grpc.coude_combats();
        let r = self
            .grpc
            .guarded(|| async move {
                client
                    .get_betting_for_participant(req)
                    .await
                    .map(|r| r.into_inner())
            })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(r.combat.map(proto_combat_to_dto))
    }

    /// Resout les paris. Retourne les resultats + bonus + TauntEvents
    /// (Migration #7 : jackpots parieurs gagnants + bonus combattants).
    pub async fn resolve_bets(
        &self,
        combat_id: &str,
        winner_id: Option<&str>,
    ) -> Result<(Vec<BetResult>, Option<FighterBetBonus>, Vec<TauntEvent>), String> {
        let req = proto_coude::ResolveBetsRequest {
            combat_id: combat_id.to_string(),
            winner_id: winner_id.map(str::to_string),
        };
        let mut client = self.grpc.coude_bets();
        let resp = self
            .grpc
            .guarded(|| async move { client.resolve(req).await.map(|r| r.into_inner()) })
            .await
            .map_err(grpc_err_to_string)?;
        let plan = resp.plan.unwrap_or_default();
        let results = plan
            .payouts
            .into_iter()
            .map(|p| BetResult {
                bettor_id: p.bettor_id,
                bettor_name: p.bettor_name,
                backed_id: p.backed_id,
                amount_bet: p.amount_bet,
                payout: p.payout,
                won: p.won,
            })
            .collect();
        let bonus = plan.fighter_bonus.map(|b| FighterBetBonus {
            winner_id: b.winner_id,
            winner_bonus: b.winner_bonus,
            loser_id: b.loser_id,
            loser_bonus: b.loser_bonus,
            total_pot: b.total_pot,
        });
        let taunts = resp
            .taunt_events
            .into_iter()
            .map(taunt_event_from_proto)
            .collect();
        Ok((results, bonus, taunts))
    }

    pub async fn refund_bets(&self, combat_id: &str) -> Result<(usize, i64), String> {
        let req = proto_coude::RefundBetsRequest {
            combat_id: combat_id.to_string(),
        };
        let mut client = self.grpc.coude_bets();
        let s = self
            .grpc
            .guarded(|| async move { client.refund(req).await.map(|r| r.into_inner()) })
            .await
            .map_err(grpc_err_to_string)?;
        Ok((s.refunded_count as usize, s.refunded_total))
    }
}
